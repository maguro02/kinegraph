use tauri::State;
use serde::{Deserialize, Serialize};
use crate::drawing_engine::{DrawingEngine, DrawStroke};
use crate::animation::Project;
use log::{info, error, debug, warn};

// 新しい描画APIモジュール
pub mod drawing;
pub use drawing::*;

#[derive(Deserialize)]
pub struct CreateProjectArgs {
    pub name: String,
    pub width: u32,
    pub height: u32,
    #[serde(alias = "frameRate")]
    pub frame_rate: f32,
}

#[derive(Deserialize)]
pub struct DrawLineArgs {
    pub layer_id: String,
    pub start_x: f32,
    pub start_y: f32,
    pub end_x: f32,
    pub end_y: f32,
    pub color: [f32; 4],
    pub width: f32,
    pub canvas_width: u32,
    pub canvas_height: u32,
}

#[derive(Deserialize)]
pub struct DrawStrokePoint {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
}

#[derive(Deserialize)]
pub struct DrawStrokeArgs {
    pub layer_id: String,
    pub points: Vec<DrawStrokePoint>,
    pub color: [f32; 4],
    pub base_width: f32,
    pub canvas_width: u32,
    pub canvas_height: u32,
}

#[derive(Serialize)]
pub struct DrawResult {
    pub success: bool,
    pub message: String,
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

#[tauri::command]
pub async fn create_layer(
    layer_id: String,
    width: u32,
    height: u32,
    drawing_engine: State<'_, std::sync::Arc<tokio::sync::Mutex<DrawingEngine>>>,
) -> Result<DrawResult, String> {
    info!("[API] create_layer コマンド呼び出し: {} ({}x{})", layer_id, width, height);
    
    let engine_arc = drawing_engine.inner();
    let mut engine = engine_arc.lock().await;
    
    match engine.create_layer_texture(&layer_id, width, height) {
        Ok(_) => {
            info!("[API] レイヤー作成成功: {}", layer_id);
            Ok(DrawResult {
                success: true,
                message: format!("レイヤー {} を作成しました", layer_id),
            })
        },
        Err(e) => {
            error!("[API] レイヤー作成失敗: {} - {}", layer_id, e);
            Err(format!("レイヤー作成エラー: {}", e))
        }
    }
}

#[tauri::command]
pub async fn draw_line(
    args: DrawLineArgs,
    drawing_engine: State<'_, std::sync::Arc<tokio::sync::Mutex<DrawingEngine>>>,
) -> Result<DrawResult, String> {
    info!("[API] draw_line コマンド呼び出し: {}", args.layer_id);
    debug!("[API] 線描画パラメータ: ({},{}) -> ({},{}) 色:{:?} 幅:{}", 
           args.start_x, args.start_y, args.end_x, args.end_y, args.color, args.width);
    
    let engine_arc = drawing_engine.inner();
    let engine = engine_arc.lock().await;
    
    // スクリーン座標を正規化座標に変換
    let start = engine.screen_to_normalized(
        (args.start_x, args.start_y), 
        (args.canvas_width, args.canvas_height)
    );
    let end = engine.screen_to_normalized(
        (args.end_x, args.end_y), 
        (args.canvas_width, args.canvas_height)
    );
    
    match engine.draw_line_to_layer(&args.layer_id, start, end, args.color, args.width) {
        Ok(_) => {
            info!("[API] 線描画成功: {}", args.layer_id);
            Ok(DrawResult {
                success: true,
                message: "線描画完了".to_string(),
            })
        },
        Err(e) => {
            error!("[API] 線描画失敗: {} - {}", args.layer_id, e);
            Err(format!("線描画エラー: {}", e))
        }
    }
}

#[tauri::command]
pub async fn draw_stroke(
    args: DrawStrokeArgs,
    drawing_engine: State<'_, std::sync::Arc<tokio::sync::Mutex<DrawingEngine>>>,
) -> Result<DrawResult, String> {
    info!("[API] draw_stroke コマンド呼び出し: {} ({} 点)", args.layer_id, args.points.len());
    
    let engine_arc = drawing_engine.inner();
    let engine = engine_arc.lock().await;
    
    // ストロークを作成
    let mut stroke = DrawStroke::new(args.color, args.base_width);
    
    for point in args.points {
        let norm_pos = engine.screen_to_normalized(
            (point.x, point.y), 
            (args.canvas_width, args.canvas_height)
        );
        stroke.add_point(norm_pos.0, norm_pos.1, point.pressure);
    }
    
    match engine.draw_stroke_to_layer(&args.layer_id, &stroke) {
        Ok(_) => {
            info!("[API] ストローク描画成功: {}", args.layer_id);
            Ok(DrawResult {
                success: true,
                message: "ストローク描画完了".to_string(),
            })
        },
        Err(e) => {
            error!("[API] ストローク描画失敗: {} - {}", args.layer_id, e);
            Err(format!("ストローク描画エラー: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_layer_data(
    layer_id: String,
    drawing_engine: State<'_, std::sync::Arc<tokio::sync::Mutex<DrawingEngine>>>,
) -> Result<Vec<u8>, String> {
    info!("[API] get_layer_data コマンド呼び出し: {}", layer_id);
    
    let engine_arc = drawing_engine.inner();
    let engine = engine_arc.lock().await;
    
    match engine.get_layer_texture_data(&layer_id).await {
        Ok(data) => {
            info!("[API] レイヤーデータ取得成功: {} ({} bytes)", layer_id, data.len());
            Ok(data)
        },
        Err(e) => {
            error!("[API] レイヤーデータ取得失敗: {} - {}", layer_id, e);
            Err(format!("レイヤーデータ取得エラー: {}", e))
        }
    }
}