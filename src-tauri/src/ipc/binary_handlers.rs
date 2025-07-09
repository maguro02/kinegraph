use tauri::{command, State};
use tauri::ipc::{InvokeBody, Request};
use crate::drawing_engine::{DrawingEngine, CanvasId};
use super::binary_protocol::{BinaryProtocol, DrawCommandBatch};
use super::simple_binary_protocol::SimpleBinaryProtocol;
use super::image_stream::{get_canvas_data_stream, get_canvas_data_chunked as get_canvas_data_chunked_impl, ImageHeader, ImageChunk};
use std::sync::Arc;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use log::{debug, info, error};

#[command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn draw_binary(
    request: Request<'_>,
    engine: State<'_, Arc<DrawingEngine>>,
) -> Result<Vec<u8>, String> {
    // バイナリデータを取得
    let data = match request.body() {
        InvokeBody::Raw(bytes) => bytes,
        _ => return Err("Expected binary data".to_string()),
    };
    
    info!("[Rust] Received binary data, size: {} bytes", data.len());
    
    // バイナリプロトコルでデコード
    let (_command_type, command) = BinaryProtocol::decode_draw_command(&data)
        .map_err(|e| format!("Failed to decode command: {}", e))?;
    
    // コマンドを処理
    engine.process_command(command).await?;
    
    // 成功レスポンスを返す（1バイト: 0 = success）
    Ok(vec![0])
}

#[command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn draw_batch_binary(
    request: Request<'_>,
    engine: State<'_, Arc<DrawingEngine>>,
) -> Result<Vec<u8>, String> {
    // バイナリデータを取得
    let data = match request.body() {
        InvokeBody::Raw(bytes) => bytes,
        _ => return Err("Expected binary data".to_string()),
    };
    
    info!("[Rust] Received batch binary data, size: {} bytes", data.len());
    
    // デバッグ用：最初の数バイトを出力
    if data.len() >= 16 {
        debug!("[Rust] First 16 bytes: {:?}", &data[..16]);
    } else {
        debug!("[Rust] Data bytes: {:?}", data);
    }
    
    // バッチコマンドをデコード
    let commands = match BinaryProtocol::decode_command_batch(&data) {
        Ok(cmds) => cmds,
        Err(e) => {
            // エラーの詳細情報を出力
            error!("[Rust] Failed to decode batch: {}", e);
            error!("[Rust] Data size: {} bytes", data.len());
            
            // 手動デコードを試みる（フォールバック）
            match bincode::deserialize::<DrawCommandBatch>(&data) {
                Ok(batch) => {
                    info!("[Rust] Manual decode successful using bincode directly");
                    match batch.to_commands() {
                        Ok(cmds) => cmds,
                        Err(e) => {
                            error!("[Rust] Failed to convert batch to commands: {}", e);
                            return Err(format!("Failed to convert batch to commands: {}", e));
                        }
                    }
                }
                Err(bincode_err) => {
                    error!("[Rust] Direct bincode deserialization also failed: {}", bincode_err);
                    // より詳細なエラー情報を提供
                    return Err(format!(
                        "Failed to decode batch. Original error: {}. Bincode error: {}. Data size: {} bytes",
                        e, bincode_err, data.len()
                    ));
                }
            }
        }
    };
    
    info!("[Rust] Decoded {} commands from batch", commands.len());
    
    // すべてのコマンドを順番に処理
    for (index, command) in commands.iter().enumerate() {
        debug!("[Rust] Processing command {}: {:?}", index, command);
        if let Err(e) = engine.process_command(command.clone()).await {
            error!("[Rust] Failed to process command {}: {}", index, e);
            return Err(format!("Failed to process command {}: {}", index, e));
        }
    }
    
    // 成功レスポンスを返す
    Ok(vec![0])
}

#[command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn get_canvas_data_binary(
    canvas_id: String,
    compressed: bool,
    engine: State<'_, Arc<DrawingEngine>>,
) -> Result<Vec<u8>, String> {
    let id = Uuid::parse_str(&canvas_id)
        .map_err(|e| format!("Invalid canvas ID: {}", e))?;
    let canvas_id = CanvasId(id);
    
    // キャンバスデータを取得
    let (image_data, width, height) = engine.get_canvas_data(&canvas_id).await?;
    
    // バイナリプロトコルでエンコード
    let encoded_data = if compressed {
        BinaryProtocol::encode_image_data_compressed(&image_data, width, height)
            .map_err(|e| format!("Compression failed: {}", e))?
    } else {
        BinaryProtocol::encode_image_data(&image_data, width, height)
    };
    
    Ok(encoded_data)
}

#[command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn get_render_result_binary(
    canvas_id: String,
    compressed: bool,
    engine: State<'_, Arc<DrawingEngine>>,
) -> Result<Vec<u8>, String> {
    let id = Uuid::parse_str(&canvas_id)
        .map_err(|e| format!("Invalid canvas ID: {}", e))?;
    let canvas_id = CanvasId(id);
    
    // キャンバスデータを取得
    let (image_data, width, height) = engine.get_canvas_data(&canvas_id).await?;
    
    // タイムスタンプを生成
    let timestamp = chrono::Utc::now().timestamp_millis() as u64;
    
    // RenderResultをバイナリプロトコルでエンコード
    let encoded_data = if compressed {
        BinaryProtocol::encode_render_result_compressed(&canvas_id.0.to_string(), &image_data, width, height, timestamp)
            .map_err(|e| format!("Compression failed: {}", e))?
    } else {
        BinaryProtocol::encode_render_result(&canvas_id.0.to_string(), &image_data, width, height, timestamp)
    };
    
    Ok(encoded_data)
}

// ストリーミング用の構造体
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct StreamingOptions {
    pub compress: bool,
    pub chunk_size: Option<usize>,
    pub use_streaming: bool,
}

#[command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn get_canvas_data_binary_stream(
    canvas_id: String,
    options: StreamingOptions,
    engine: State<'_, Arc<DrawingEngine>>,
) -> Result<Vec<u8>, String> {
    info!("Getting canvas data with streaming options: {:?}", options);
    
    let id = Uuid::parse_str(&canvas_id)
        .map_err(|e| format!("Invalid canvas ID: {}", e))?;
    let canvas_id = CanvasId(id);
    
    // ストリーミング実装を使用
    let (header, data) = get_canvas_data_stream(
        engine.inner().clone(),
        &canvas_id,
        options.compress,
        options.chunk_size,
    ).await?;
    
    // ヘッダーと画像データを結合して返す
    let header_json = serde_json::to_vec(&header)
        .map_err(|e| format!("Failed to serialize header: {}", e))?;
    
    // ヘッダー長（4バイト）+ ヘッダー + 画像データ
    let header_len = header_json.len() as u32;
    let mut result = Vec::with_capacity(4 + header_json.len() + data.len());
    
    result.extend_from_slice(&header_len.to_le_bytes());
    result.extend_from_slice(&header_json);
    result.extend_from_slice(&data);
    
    debug!("Streaming canvas data: header={} bytes, data={} bytes", 
           header_json.len(), data.len());
    
    Ok(result)
}

#[command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn get_canvas_data_chunked(
    canvas_id: String,
    chunk_size: usize,
    compress: bool,
    engine: State<'_, Arc<DrawingEngine>>,
) -> Result<(ImageHeader, Vec<ImageChunk>), String> {
    info!("Getting canvas data in chunks: chunk_size={}, compress={}", 
          chunk_size, compress);
    
    let id = Uuid::parse_str(&canvas_id)
        .map_err(|e| format!("Invalid canvas ID: {}", e))?;
    let canvas_id = CanvasId(id);
    
    let (header, chunks) = get_canvas_data_chunked_impl(
        engine.inner().clone(),
        &canvas_id,
        compress,
        chunk_size,
    ).await?;
    
    debug!("Canvas data split into {} chunks", chunks.len());
    
    Ok((header, chunks))
}

// シンプルバイナリプロトコル用ハンドラー
#[command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn simple_draw_command(
    request: Request<'_>,
    engine: State<'_, Arc<DrawingEngine>>,
) -> Result<Vec<u8>, String> {
    // バイナリデータを取得
    let data = match request.body() {
        InvokeBody::Raw(bytes) => bytes,
        _ => return Err("Expected binary data".to_string()),
    };
    
    debug!("[RS] simple_draw_command received {} bytes", data.len());
    
    // シンプルバイナリプロトコルでデコード
    let command = SimpleBinaryProtocol::decode_command(&data)
        .map_err(|e| format!("Failed to decode simple command: {}", e))?;
    
    debug!("[RS] Decoded command: {:?}", command);
    
    // コマンドを処理
    engine.process_command(command).await?;
    
    // 成功レスポンスを返す（1バイト: 0 = success）
    Ok(vec![0])
}

#[command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn simple_draw_batch(
    request: Request<'_>,
    engine: State<'_, Arc<DrawingEngine>>,
) -> Result<Vec<u8>, String> {
    // バイナリデータを取得
    let data = match request.body() {
        InvokeBody::Raw(bytes) => bytes,
        _ => return Err("Expected binary data".to_string()),
    };
    
    debug!("[RS] simple_draw_batch received {} bytes", data.len());
    
    // シンプルバイナリプロトコルでバッチをデコード
    let commands = SimpleBinaryProtocol::decode_commands(&data)
        .map_err(|e| format!("Failed to decode simple batch: {}", e))?;
    
    debug!("[RS] Decoded {} commands from simple batch", commands.len());
    
    // すべてのコマンドを順番に処理
    for (index, command) in commands.iter().enumerate() {
        debug!("[RS] Processing simple command {}: {:?}", index, command);
        if let Err(e) = engine.process_command(command.clone()).await {
            error!("[RS] Failed to process simple command {}: {}", index, e);
            return Err(format!("Failed to process command {}: {}", index, e));
        }
    }
    
    // 成功レスポンスを返す
    Ok(vec![0])
}

// パフォーマンス比較用：従来のJSON版とバイナリ版の切り替え
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PerformanceTestOptions {
    pub use_binary: bool,
    pub compress: bool,
    pub measure_time: bool,
}

#[command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn get_canvas_data_perf_test(
    canvas_id: String,
    options: PerformanceTestOptions,
    engine: State<'_, Arc<DrawingEngine>>,
) -> Result<Vec<u8>, String> {
    use std::time::Instant;
    
    let start = if options.measure_time {
        Some(Instant::now())
    } else {
        None
    };
    
    let id = Uuid::parse_str(&canvas_id)
        .map_err(|e| format!("Invalid canvas ID: {}", e))?;
    let canvas_id = CanvasId(id);
    
    // データ取得
    let fetch_start = Instant::now();
    let (image_data, width, height) = engine.get_canvas_data(&canvas_id).await?;
    let fetch_time = fetch_start.elapsed();
    
    // エンコード
    let encode_start = Instant::now();
    let result = if options.use_binary {
        // バイナリエンコード
        if options.compress {
            BinaryProtocol::encode_image_data_compressed(&image_data, width, height)
                .map_err(|e| format!("Compression failed: {}", e))?
        } else {
            BinaryProtocol::encode_image_data(&image_data, width, height)
        }
    } else {
        // JSON エンコード（比較用）
        let json_data = serde_json::json!({
            "width": width,
            "height": height,
            "data": image_data,
        });
        serde_json::to_vec(&json_data)
            .map_err(|e| format!("JSON encoding failed: {}", e))?
    };
    let encode_time = encode_start.elapsed();
    
    if let Some(start_time) = start {
        let total_time = start_time.elapsed();
        info!("Performance test results:");
        info!("  - Fetch time: {:?}", fetch_time);
        info!("  - Encode time: {:?}", encode_time);
        info!("  - Total time: {:?}", total_time);
        info!("  - Data size: {} bytes", result.len());
        info!("  - Format: {}", if options.use_binary { "binary" } else { "json" });
        info!("  - Compressed: {}", options.compress);
    }
    
    Ok(result)
}

/// シンプルなバイナリプロトコルを使用した描画コマンド処理
#[command]
pub async fn draw_command_simple_binary(
    request: Request<'_>,
    engine: State<'_, Arc<DrawingEngine>>,
) -> Result<(), String> {
    // バイナリデータを取得
    let data = match request.body() {
        InvokeBody::Raw(bytes) => bytes,
        _ => return Err("Expected binary data".to_string()),
    };
    
    info!("[Rust] Received simple binary command, size: {} bytes", data.len());
    
    // シンプルバイナリプロトコルでデコード
    let command = SimpleBinaryProtocol::decode_command(&data)
        .map_err(|e| format!("Failed to decode command: {}", e))?;
    
    debug!("[Rust] Decoded command: {:?}", command);
    
    // コマンドを処理
    engine.process_command(command).await?;
    
    Ok(())
}

/// シンプルなバイナリプロトコルを使用したバッチコマンド処理
#[command]
pub async fn draw_batch_simple_binary(
    request: Request<'_>,
    engine: State<'_, Arc<DrawingEngine>>,
) -> Result<(), String> {
    // バイナリデータを取得
    let data = match request.body() {
        InvokeBody::Raw(bytes) => bytes,
        _ => return Err("Expected binary data".to_string()),
    };
    
    info!("[Rust] Received simple binary batch, size: {} bytes", data.len());
    
    // シンプルバイナリプロトコルでデコード
    let commands = SimpleBinaryProtocol::decode_commands(&data)
        .map_err(|e| format!("Failed to decode commands: {}", e))?;
    
    info!("[Rust] Decoded {} commands", commands.len());
    
    // コマンドを処理
    for command in commands {
        engine.process_command(command).await?;
    }
    
    Ok(())
}