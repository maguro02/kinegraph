use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use log::{info, error, debug};
use crate::drawing_engine::{DrawEngineCommand, DrawingEngine, CanvasId};

/// ワーカースレッドへのメッセージ
#[derive(Debug)]
pub enum WorkerMessage {
    /// 描画コマンドの実行
    DrawCommand {
        canvas_id: CanvasId,
        command: DrawEngineCommand,
        /// レスポンスを送信するためのチャネル
        response_tx: mpsc::Sender<Result<(), String>>,
    },
    /// バッチコマンドの実行
    DrawCommandBatch {
        canvas_id: CanvasId,
        commands: Vec<DrawEngineCommand>,
        response_tx: mpsc::Sender<Result<(), String>>,
    },
    /// ワーカーのシャットダウン
    Shutdown,
}

/// 描画エンジンワーカー
pub struct DrawingWorker {
    /// メッセージ受信チャネル
    rx: mpsc::Receiver<WorkerMessage>,
    /// 描画エンジンへの共有参照
    engine: Arc<RwLock<DrawingEngine>>,
}

impl DrawingWorker {
    /// 新しいワーカーを作成
    pub fn new(
        rx: mpsc::Receiver<WorkerMessage>,
        engine: Arc<RwLock<DrawingEngine>>,
    ) -> Self {
        Self { rx, engine }
    }

    /// ワーカーを実行
    pub async fn run(mut self) {
        info!("Drawing worker started");

        while let Some(msg) = self.rx.recv().await {
            match msg {
                WorkerMessage::DrawCommand {
                    canvas_id,
                    command,
                    response_tx,
                } => {
                    debug!("Processing draw command for canvas {:?}", canvas_id);
                    let result = self.process_command(canvas_id, command).await;
                    
                    // エラーの場合のみレスポンスを送信（送信エラーは無視）
                    if let Err(e) = result {
                        let _ = response_tx.send(Err(e)).await;
                    } else {
                        let _ = response_tx.send(Ok(())).await;
                    }
                }
                WorkerMessage::DrawCommandBatch {
                    canvas_id,
                    commands,
                    response_tx,
                } => {
                    debug!("Processing batch of {} commands for canvas {:?}", commands.len(), canvas_id);
                    let result = self.process_command_batch(canvas_id, commands).await;
                    
                    // バッチ処理の結果を送信
                    let _ = response_tx.send(result).await;
                }
                WorkerMessage::Shutdown => {
                    info!("Drawing worker shutting down");
                    break;
                }
            }
        }

        info!("Drawing worker stopped");
    }

    /// 単一のコマンドを処理
    async fn process_command(
        &self,
        canvas_id: CanvasId,
        command: DrawEngineCommand,
    ) -> Result<(), String> {
        let engine = self.engine.read().await;
        match engine.process_command_with_canvas(&canvas_id, command).await {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed to execute command: {}", e);
                Err(format!("Command execution failed: {}", e))
            }
        }
    }

    /// バッチコマンドを処理
    async fn process_command_batch(
        &self,
        canvas_id: CanvasId,
        commands: Vec<DrawEngineCommand>,
    ) -> Result<(), String> {
        let mut errors = Vec::new();

        // バッチ内のコマンドを順次実行
        for (i, command) in commands.into_iter().enumerate() {
            let engine = self.engine.read().await;
            if let Err(e) = engine.process_command_with_canvas(&canvas_id, command).await {
                error!("Failed to execute command {} in batch: {}", i, e);
                errors.push(format!("Command {} failed: {}", i, e));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }
}

/// ワーカープール管理
pub struct WorkerPool {
    /// ワーカーへの送信チャネル
    tx: mpsc::Sender<WorkerMessage>,
    /// ワーカータスクのハンドル
    worker_handle: Option<tokio::task::JoinHandle<()>>,
}

impl WorkerPool {
    /// 新しいワーカープールを作成
    pub fn new(engine: Arc<RwLock<DrawingEngine>>) -> Self {
        let (tx, rx) = mpsc::channel(1000); // バッファサイズ1000のチャネル
        
        // ワーカータスクを起動
        let worker = DrawingWorker::new(rx, engine);
        let worker_handle = tokio::spawn(async move {
            worker.run().await;
        });

        Self {
            tx,
            worker_handle: Some(worker_handle),
        }
    }

    /// 描画コマンドを送信
    pub async fn send_command(
        &self,
        canvas_id: CanvasId,
        command: DrawEngineCommand,
    ) -> Result<(), String> {
        let (response_tx, mut response_rx) = mpsc::channel(1);

        self.tx
            .send(WorkerMessage::DrawCommand {
                canvas_id,
                command,
                response_tx,
            })
            .await
            .map_err(|_| "Failed to send command to worker".to_string())?;

        // レスポンスを待つ（タイムアウト付き）
        match tokio::time::timeout(
            std::time::Duration::from_millis(100),
            response_rx.recv(),
        )
        .await
        {
            Ok(Some(result)) => result,
            Ok(None) => Err("Worker response channel closed".to_string()),
            Err(_) => Err("Worker response timeout".to_string()),
        }
    }

    /// バッチコマンドを送信
    pub async fn send_command_batch(
        &self,
        canvas_id: CanvasId,
        commands: Vec<DrawEngineCommand>,
    ) -> Result<(), String> {
        let (response_tx, mut response_rx) = mpsc::channel(1);

        self.tx
            .send(WorkerMessage::DrawCommandBatch {
                canvas_id,
                commands,
                response_tx,
            })
            .await
            .map_err(|_| "Failed to send batch to worker".to_string())?;

        // バッチ処理は時間がかかる可能性があるため、タイムアウトを長めに設定
        match tokio::time::timeout(
            std::time::Duration::from_millis(500),
            response_rx.recv(),
        )
        .await
        {
            Ok(Some(result)) => result,
            Ok(None) => Err("Worker response channel closed".to_string()),
            Err(_) => Err("Worker batch response timeout".to_string()),
        }
    }

    /// ワーカープールをシャットダウン
    pub async fn shutdown(mut self) {
        // シャットダウンメッセージを送信
        let _ = self.tx.send(WorkerMessage::Shutdown).await;

        // ワーカータスクの完了を待つ
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawing_engine::{DrawingEngine, BrushSettings};

    #[tokio::test]
    async fn test_worker_basic_operation() {
        // テスト用の描画エンジンを作成
        let engine = Arc::new(RwLock::new(DrawingEngine::new().await.unwrap()));
        
        // ワーカープールを作成
        let pool = WorkerPool::new(engine.clone());
        
        // キャンバスを初期化
        let canvas_id = {
            let mut engine = engine.write().await;
            engine.create_canvas(800, 600).await.unwrap()
        };
        
        // ブラシ設定コマンドを送信
        let brush_settings = BrushSettings {
            size: 10.0,
            color: [0.0, 0.0, 0.0, 1.0],
            opacity: 1.0,
            brush_type: crate::drawing_engine::commands::BrushType::Pen,
            blend_mode: crate::drawing_engine::commands::BlendMode::Normal,
        };
        
        let result = pool.send_command(
            canvas_id.clone(),
            DrawEngineCommand::SetBrush(brush_settings),
        ).await;
        
        assert!(result.is_ok());
        
        // シャットダウン
        pool.shutdown().await;
    }

    #[tokio::test]
    async fn test_worker_batch_processing() {
        let engine = Arc::new(RwLock::new(DrawingEngine::new().await.unwrap()));
        let pool = WorkerPool::new(engine.clone());
        
        let canvas_id = {
            let mut engine = engine.write().await;
            engine.create_canvas(800, 600).await.unwrap()
        };
        
        // バッチコマンドを作成
        let commands = vec![
            DrawEngineCommand::BeginStroke { x: 10.0, y: 10.0, pressure: 1.0 },
            DrawEngineCommand::ContinueStroke { x: 20.0, y: 20.0, pressure: 1.0 },
            DrawEngineCommand::ContinueStroke { x: 30.0, y: 30.0, pressure: 1.0 },
            DrawEngineCommand::EndStroke,
        ];
        
        let result = pool.send_command_batch(canvas_id, commands).await;
        assert!(result.is_ok());
        
        pool.shutdown().await;
    }
}