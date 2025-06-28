use tauri::State;
use serde::Deserialize;
use crate::drawing_engine::DrawingEngine;
use crate::animation::Project;
use log::{info, error, debug, warn};

#[derive(Deserialize)]
pub struct CreateProjectArgs {
    pub name: String,
    pub width: u32,
    pub height: u32,
    #[serde(alias = "frameRate")]
    pub frame_rate: f32,
}

#[tauri::command]
pub async fn create_project(
    args: CreateProjectArgs,
    drawing_engine: State<'_, std::sync::Arc<tokio::sync::Mutex<DrawingEngine>>>,
) -> Result<Project, String> {
    info!("[API] create_project コマンド呼び出し開始");
    debug!("[API] プロジェクトパラメータ: name={}, width={}, height={}, frame_rate={}", 
           args.name, args.width, args.height, args.frame_rate);
    
    // 状態管理からDrawingEngineを取得しようと試みる
    debug!("[API] Tauri State から DrawingEngine を取得中...");
    
    // 状態が管理されているかの確認（inner()は常に成功する）
    let engine_arc = drawing_engine.inner();
    debug!("[API] DrawingEngine 状態取得成功");
    
    debug!("[API] DrawingEngine ミューテックスロック取得中...");
    let mut engine = match engine_arc.try_lock() {
        Ok(engine) => {
            debug!("[API] DrawingEngine ミューテックスロック取得成功");
            engine
        },
        Err(e) => {
            warn!("[API] DrawingEngine ミューテックスロック取得失敗、await で再試行: {}", e);
            engine_arc.lock().await
        }
    };
    
    debug!("[API] DrawingEngine 初期化開始...");
    match engine.initialize().await {
        Ok(_) => {
            info!("[API] DrawingEngine 初期化成功");
        },
        Err(e) => {
            error!("[API] DrawingEngine 初期化失敗: {}", e);
            return Err(format!("DrawingEngine 初期化エラー: {}", e));
        }
    }
    
    debug!("[API] Project インスタンス作成中...");
    let project = Project::new(args.name.clone(), args.width, args.height, args.frame_rate);
    info!("[API] create_project コマンド正常完了: {}", args.name);
    
    Ok(project)
}

#[tauri::command]
pub async fn get_system_info() -> Result<String, String> {
    info!("[API] get_system_info コマンド呼び出し");
    let info = format!(
        "Drawing app initialized - Tauri + wgpu + React + jotai"
    );
    debug!("[API] システム情報: {}", info);
    Ok(info)
}