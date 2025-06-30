use crate::drawing_engine::{DrawingEngine, DrawStroke, Vertex2D};
use log::{info, debug, warn, error, trace};
use std::collections::HashMap;
use tokio::sync::Mutex;
use tauri::{State, AppHandle, Emitter};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 描画更新イベント
#[derive(Debug, Clone, Serialize)]
pub struct DrawingUpdate {
    pub layer_id: String,
    pub update_type: UpdateType,
    pub data: Vec<u8>,
    pub rect: UpdateRect,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateType {
    StrokeProgress,  // ストローク描画中
    StrokeComplete,  // ストローク完了
    PartialUpdate,   // 部分更新
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// アクティブなストローク情報
#[derive(Debug)]
struct ActiveStroke {
    id: String,
    layer_id: String,
    color: [f32; 4],
    points: Vec<StrokePoint>,
    last_update_index: usize,
}

/// 描画エンジンの状態管理
pub struct DrawingState {
    engine: Mutex<Option<DrawingEngine>>,
    layers: Mutex<HashMap<String, (u32, u32)>>, // layer_id -> (width, height)
    active_strokes: Mutex<HashMap<String, ActiveStroke>>, // stroke_id -> ActiveStroke
}

impl DrawingState {
    pub fn new() -> Self {
        info!("[Drawing State] 新しい描画状態を初期化");
        Self {
            engine: Mutex::new(None),
            layers: Mutex::new(HashMap::new()),
            active_strokes: Mutex::new(HashMap::new()),
        }
    }

    /// デバッグ用：現在の状態を詳細出力
    pub async fn log_detailed_state(&self) {
        let engine_initialized = {
            let engine_guard = self.engine.lock().await;
            engine_guard.is_some()
        };
        
        let layers_info = {
            let layers_guard = self.layers.lock().await;
            layers_guard.len()
        };
        
        debug!("[Drawing State] エンジン初期化: {}, レイヤー数: {}", 
               engine_initialized, layers_info);
    }
}

/// 描画エンジンを初期化
#[tauri::command]
pub async fn initialize_drawing_engine(
    state: State<'_, DrawingState>,
) -> Result<String, String> {
    info!("[Drawing API] 描画エンジン初期化開始");
    trace!("[Drawing API] 初期化前の状態確認");
    
    // 現在の状態をログ出力
    state.log_detailed_state().await;
    
    // 重複初期化チェック
    {
        let engine_guard = state.engine.lock().await;
        if engine_guard.is_some() {
            warn!("[Drawing API] 描画エンジンは既に初期化済み - スキップ");
            return Ok("描画エンジンは既に初期化されています".to_string());
        }
        debug!("[Drawing API] エンジン未初期化を確認 - 初期化を続行");
    }
    
    // 描画エンジン作成
    debug!("[Drawing API] DrawingEngine::new() を呼び出し");
    let mut engine = DrawingEngine::new();
    
    // 初期化実行
    debug!("[Drawing API] engine.initialize() を実行開始");
    match engine.initialize().await {
        Ok(_) => {
            debug!("[Drawing API] engine.initialize() が正常完了");
        },
        Err(e) => {
            error!("[Drawing API] engine.initialize() でエラー発生: {}", e);
            return Err(format!("初期化エラー: {}", e));
        }
    }
    
    // エンジンを状態に設定
    debug!("[Drawing API] 初期化済みエンジンを状態に保存");
    {
        let mut engine_guard = state.engine.lock().await;
        *engine_guard = Some(engine);
    }
    
    // 最終状態確認
    state.log_detailed_state().await;
    info!("[Drawing API] 描画エンジン初期化完了");
    Ok("描画エンジンが正常に初期化されました".to_string())
}

/// レイヤーを作成
#[tauri::command]
pub async fn create_drawing_layer(
    layer_id: String,
    width: u32,
    height: u32,
    state: State<'_, DrawingState>,
) -> Result<String, String> {
    info!("[Drawing API] レイヤー作成開始");
    debug!("[Drawing API] 引数受信確認 - layer_id: '{}', width: {}, height: {}", 
           layer_id, width, height);
    
    // 引数バリデーション
    if layer_id.is_empty() {
        error!("[Drawing API] レイヤーIDが空です");
        return Err("レイヤーIDが空です".to_string());
    }
    
    if width == 0 || height == 0 {
        error!("[Drawing API] 無効な解像度: {}x{}", width, height);
        return Err("解像度は1以上である必要があります".to_string());
    }
    
    // 最大解像度チェック
    if width > 4096 || height > 4096 {
        error!("[Drawing API] 解像度上限超過: {}x{} (最大: 4096x4096)", width, height);
        return Err("解像度が最大値(4096x4096)を超えています".to_string());
    }
    
    debug!("[Drawing API] 引数バリデーション完了");
    
    // 現在の状態をログ出力
    state.log_detailed_state().await;
    
    // 重複レイヤーチェック
    {
        let layers_guard = state.layers.lock().await;
        if layers_guard.contains_key(&layer_id) {
            warn!("[Drawing API] レイヤーID重複: {} - 既存レイヤーを上書き", layer_id);
        } else {
            debug!("[Drawing API] レイヤーID重複なし - 作成続行");
        }
        debug!("[Drawing API] 現在のレイヤー数: {}", layers_guard.len());
    }
    
    // 描画エンジンでのレイヤー作成
    debug!("[Drawing API] 描画エンジンでレイヤーテクスチャ作成開始");
    {
        let mut engine_guard = state.engine.lock().await;
        match engine_guard.as_mut() {
            Some(engine) => {
                debug!("[Drawing API] 描画エンジン取得成功 - create_layer_texture呼び出し");
                match engine.create_layer_texture(&layer_id, width, height) {
                    Ok(_) => {
                        debug!("[Drawing API] create_layer_texture が正常完了");
                    },
                    Err(e) => {
                        error!("[Drawing API] create_layer_texture でエラー: {}", e);
                        return Err(format!("レイヤー作成エラー: {}", e));
                    }
                }
            },
            None => {
                error!("[Drawing API] 描画エンジンが初期化されていません");
                return Err("描画エンジンが初期化されていません".to_string());
            }
        }
    }
    
    // レイヤー情報を状態に保存
    debug!("[Drawing API] レイヤー情報を状態に保存");
    {
        let mut layers_guard = state.layers.lock().await;
        layers_guard.insert(layer_id.clone(), (width, height));
        debug!("[Drawing API] レイヤー情報保存完了 - 総レイヤー数: {}", layers_guard.len());
    }
    
    // 最終状態確認
    state.log_detailed_state().await;
    info!("[Drawing API] レイヤー作成完了: {} ({}x{})", layer_id, width, height);
    Ok(layer_id)
}

/// レイヤーに線を描画
#[tauri::command]
pub async fn draw_line_on_layer(
    layer_id: String,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    color: [f32; 4],
    width: f32,
    state: State<'_, DrawingState>,
) -> Result<(), String> {
    info!("[Drawing API] 線描画開始");
    debug!("[Drawing API] 線描画パラメータ: layer_id='{}', 開始点=({},{}), 終了点=({},{}), 色={:?}, 幅={}", 
           layer_id, x1, y1, x2, y2, color, width);
    
    // パラメータ検証
    if layer_id.is_empty() {
        error!("[Drawing API] レイヤーIDが空です");
        return Err("レイヤーIDが空です".to_string());
    }
    
    if width <= 0.0 {
        error!("[Drawing API] 無効な線幅: {}", width);
        return Err("線幅は0より大きい値である必要があります".to_string());
    }
    
    // レイヤーの存在確認
    let (layer_width, layer_height) = {
        let layers_guard = state.layers.lock().await;
        match layers_guard.get(&layer_id) {
            Some(dimensions) => {
                debug!("[Drawing API] レイヤー確認OK: {} ({}x{})", layer_id, dimensions.0, dimensions.1);
                dimensions.clone()
            },
            None => {
                error!("[Drawing API] レイヤーが見つかりません: {}", layer_id);
                return Err(format!("レイヤーが見つかりません: {}", layer_id));
            }
        }
    };
    
    // 線を描画
    debug!("[Drawing API] 描画エンジンでの線描画処理開始");
    {
        let engine_guard = state.engine.lock().await;
        match engine_guard.as_ref() {
            Some(engine) => {
                debug!("[Drawing API] 描画エンジン取得成功");
                
                // スクリーン座標を正規化座標に変換
                debug!("[Drawing API] 座標変換開始");
                let start_norm = engine.screen_to_normalized((x1, y1), (layer_width, layer_height));
                let end_norm = engine.screen_to_normalized((x2, y2), (layer_width, layer_height));
                debug!("[Drawing API] 座標変換完了: ({:.3},{:.3}) -> ({:.3},{:.3})", 
                       start_norm.0, start_norm.1, end_norm.0, end_norm.1);
                
                // 線を描画
                debug!("[Drawing API] draw_line_to_layer呼び出し");
                match engine.draw_line_to_layer(&layer_id, start_norm, end_norm, color, width) {
                    Ok(_) => {
                        debug!("[Drawing API] draw_line_to_layer成功");
                    },
                    Err(e) => {
                        error!("[Drawing API] draw_line_to_layerでエラー: {}", e);
                        return Err(format!("線描画エラー: {}", e));
                    }
                }
            },
            None => {
                error!("[Drawing API] 描画エンジンが初期化されていません");
                return Err("描画エンジンが初期化されていません".to_string());
            }
        }
    }
    
    info!("[Drawing API] 線描画完了: {}", layer_id);
    Ok(())
}

/// レイヤーにストロークを描画（筆圧対応）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrokePoint {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
}

#[tauri::command]
pub async fn draw_stroke_on_layer(
    layer_id: String,
    points: Vec<StrokePoint>,
    color: [f32; 4],
    state: State<'_, DrawingState>,
) -> Result<(), String> {
    debug!("[Drawing API] ストローク描画: {} ({} 点)", layer_id, points.len());
    
    if points.is_empty() {
        return Err("ストロークの点が空です".to_string());
    }
    
    // レイヤーの存在確認
    let (layer_width, layer_height) = {
        let layers_guard = state.layers.lock().await;
        layers_guard.get(&layer_id)
            .ok_or(format!("レイヤーが見つかりません: {}", layer_id))?
            .clone()
    };
    
    // ストロークを描画
    {
        let engine_guard = state.engine.lock().await;
        let engine = engine_guard.as_ref().ok_or("描画エンジンが初期化されていません")?;
        
        // スクリーン座標を正規化座標に変換してVertex2Dを作成
        let vertex_points: Vec<Vertex2D> = points.iter().map(|p| {
            let norm_pos = engine.screen_to_normalized((p.x, p.y), (layer_width, layer_height));
            Vertex2D::new(norm_pos.0, norm_pos.1, color, 2.0 * p.pressure) // 筆圧で線幅調整
        }).collect();
        
        // ストロークを作成
        let stroke = DrawStroke {
            points: vertex_points,
            color,
            base_width: 2.0, // デフォルト線幅
            is_closed: false, // 通常のストロークは閉じない
        };
        
        // ストロークを描画
        engine.draw_stroke_to_layer(&layer_id, &stroke)
            .map_err(|e| format!("ストローク描画エラー: {}", e))?;
    }
    
    info!("[Drawing API] ストローク描画完了: {}", layer_id);
    Ok(())
}

/// レイヤーの画像データを取得
#[tauri::command]
pub async fn get_layer_image_data(
    layer_id: String,
    state: State<'_, DrawingState>,
) -> Result<Vec<u8>, String> {
    debug!("[Drawing API] レイヤー画像データ取得: {}", layer_id);
    
    // レイヤーの存在確認
    {
        let layers_guard = state.layers.lock().await;
        if !layers_guard.contains_key(&layer_id) {
            return Err(format!("レイヤーが見つかりません: {}", layer_id));
        }
    }
    
    // 画像データを取得
    let image_data = {
        let engine_guard = state.engine.lock().await;
        let engine = engine_guard.as_ref().ok_or("描画エンジンが初期化されていません")?;
        
        engine.get_layer_texture_data(&layer_id).await
            .map_err(|e| format!("画像データ取得エラー: {}", e))?
    };
    
    info!("[Drawing API] レイヤー画像データ取得完了: {} ({} バイト)", layer_id, image_data.len());
    Ok(image_data)
}

/// レイヤーをクリア
#[tauri::command]
pub async fn clear_layer(
    layer_id: String,
    state: State<'_, DrawingState>,
) -> Result<(), String> {
    debug!("[Drawing API] レイヤークリア: {}", layer_id);
    
    // レイヤーの存在確認
    {
        let layers_guard = state.layers.lock().await;
        if !layers_guard.contains_key(&layer_id) {
            return Err(format!("レイヤーが見つかりません: {}", layer_id));
        }
    }
    
    // レイヤーをクリア（透明）
    {
        let mut engine_guard = state.engine.lock().await;
        let engine = engine_guard.as_mut().ok_or("描画エンジンが初期化されていません")?;
        
        engine.clear_layer_texture(&layer_id, Some(wgpu::Color::TRANSPARENT))
            .map_err(|e| format!("レイヤークリアエラー: {}", e))?;
    }
    
    info!("[Drawing API] レイヤークリア完了: {}", layer_id);
    Ok(())
}

/// レイヤーを削除
#[tauri::command]
pub async fn remove_layer(
    layer_id: String,
    state: State<'_, DrawingState>,
) -> Result<(), String> {
    debug!("[Drawing API] レイヤー削除: {}", layer_id);
    
    // レイヤーテクスチャを削除
    let removed = {
        let mut engine_guard = state.engine.lock().await;
        let engine = engine_guard.as_mut().ok_or("描画エンジンが初期化されていません")?;
        engine.remove_layer_texture(&layer_id)
    };
    
    if removed {
        // レイヤー情報も削除
        {
            let mut layers_guard = state.layers.lock().await;
            layers_guard.remove(&layer_id);
        }
        
        info!("[Drawing API] レイヤー削除完了: {}", layer_id);
        Ok(())
    } else {
        Err(format!("レイヤーが見つかりません: {}", layer_id))
    }
}

/// リアルタイムストローク描画を開始
#[tauri::command]
pub async fn begin_realtime_stroke(
    app: AppHandle,
    layer_id: String,
    color: [f32; 4],
    _brush_size: f32,
    tool: String,
    state: State<'_, DrawingState>,
) -> Result<String, String> {
    debug!("[Drawing API] リアルタイムストローク開始: layer={}, tool={}", layer_id, tool);
    
    // レイヤーの存在確認
    {
        let layers_guard = state.layers.lock().await;
        if !layers_guard.contains_key(&layer_id) {
            return Err(format!("レイヤーが見つかりません: {}", layer_id));
        }
    }
    
    // 新しいストロークIDを生成
    let stroke_id = Uuid::new_v4().to_string();
    
    // アクティブストロークとして登録
    {
        let mut strokes_guard = state.active_strokes.lock().await;
        strokes_guard.insert(stroke_id.clone(), ActiveStroke {
            id: stroke_id.clone(),
            layer_id: layer_id.clone(),
            color,
            points: Vec::new(),
            last_update_index: 0,
        });
    }
    
    // フロントエンドに通知
    app.emit("stroke-started", &stroke_id)
        .map_err(|e| format!("イベント送信エラー: {}", e))?;
    
    info!("[Drawing API] リアルタイムストローク開始: id={}", stroke_id);
    Ok(stroke_id)
}

/// リアルタイムストロークに点を追加
#[tauri::command]
pub async fn add_realtime_stroke_point(
    app: AppHandle,
    stroke_id: String,
    point: StrokePoint,
    state: State<'_, DrawingState>,
) -> Result<(), String> {
    trace!("[Drawing API] ストローク点追加: stroke={}, point=({}, {})", 
        stroke_id, point.x, point.y);
    
    let (layer_id, should_update) = {
        let mut strokes_guard = state.active_strokes.lock().await;
        let stroke = strokes_guard.get_mut(&stroke_id)
            .ok_or_else(|| format!("アクティブストロークが見つかりません: {}", stroke_id))?;
        
        stroke.points.push(point.clone());
        let layer_id = stroke.layer_id.clone();
        
        // 一定数の点が追加されたら更新する（パフォーマンス調整）
        let should_update = stroke.points.len() - stroke.last_update_index >= 5;
        if should_update {
            stroke.last_update_index = stroke.points.len();
        }
        
        (layer_id, should_update)
    };
    
    // 描画エンジンで部分的に描画し、更新をフロントエンドに送信
    if should_update {
        let update_rect = calculate_update_rect(&point);
        
        // 部分的な描画処理
        let partial_data = {
            let mut engine_guard = state.engine.lock().await;
            let engine = engine_guard.as_mut()
                .ok_or("描画エンジンが初期化されていません")?;
            
            // ストロークの最新部分のみを描画
            let strokes_guard = state.active_strokes.lock().await;
            let stroke = strokes_guard.get(&stroke_id).unwrap();
            
            // 最後の更新以降の点のみを描画
            let recent_points = &stroke.points[stroke.last_update_index..];
            if recent_points.len() >= 2 {
                let partial_stroke = DrawStroke {
                    color: stroke.color,
                    points: recent_points.iter()
                        .map(|p| {
                            let ndc_x = (p.x / 1920.0) * 2.0 - 1.0;
                            let ndc_y = -((p.y / 1080.0) * 2.0 - 1.0);
                            Vertex2D::new(ndc_x, ndc_y, stroke.color, p.pressure * 10.0)
                        })
                        .collect(),
                    base_width: 10.0,
                    is_closed: false,
                };
                
                engine.draw_stroke_to_layer(&layer_id, &partial_stroke)
                    .map_err(|e| format!("部分描画エラー: {}", e))?;
            }
            
            // 更新領域のピクセルデータを取得
            get_partial_texture_data(engine, &layer_id, &update_rect).await?
        };
        
        // リアルタイム更新イベントを送信
        let update = DrawingUpdate {
            layer_id,
            update_type: UpdateType::StrokeProgress,
            data: partial_data,
            rect: update_rect,
        };
        
        app.emit("drawing-update", &update)
            .map_err(|e| format!("更新イベント送信エラー: {}", e))?;
    }
    
    Ok(())
}

/// リアルタイムストロークを完了
#[tauri::command]
pub async fn complete_realtime_stroke(
    app: AppHandle,
    stroke_id: String,
    state: State<'_, DrawingState>,
) -> Result<(), String> {
    debug!("[Drawing API] リアルタイムストローク完了: {}", stroke_id);
    
    // アクティブストロークを取得して削除
    let stroke_data = {
        let mut strokes_guard = state.active_strokes.lock().await;
        strokes_guard.remove(&stroke_id)
            .ok_or_else(|| format!("アクティブストロークが見つかりません: {}", stroke_id))?
    };
    
    // 完了通知を送信
    app.emit("stroke-completed", &stroke_id)
        .map_err(|e| format!("完了イベント送信エラー: {}", e))?;
    
    info!("[Drawing API] リアルタイムストローク完了: id={}, points={}", 
        stroke_id, stroke_data.points.len());
    Ok(())
}

/// 更新領域を計算
fn calculate_update_rect(point: &StrokePoint) -> UpdateRect {
    // ブラシサイズを考慮して更新領域を計算
    let brush_radius = (point.pressure * 20.0).ceil() as u32;
    let x = (point.x - brush_radius as f32).max(0.0) as u32;
    let y = (point.y - brush_radius as f32).max(0.0) as u32;
    let width = (brush_radius * 2) + 1;
    let height = (brush_radius * 2) + 1;
    
    UpdateRect { x, y, width, height }
}

/// 部分的なテクスチャデータを取得
async fn get_partial_texture_data(
    engine: &mut DrawingEngine,
    layer_id: &str,
    _rect: &UpdateRect,
) -> Result<Vec<u8>, String> {
    // TODO: 実際の部分データ取得を実装
    // 現時点では全体データを返す（最適化は後で実装）
    engine.get_layer_texture_data(layer_id).await
        .map_err(|e| format!("部分データ取得エラー: {}", e))
}

/// 描画エンジンの統計情報を取得
#[derive(Serialize)]
pub struct DrawingStats {
    pub layers_count: usize,
    pub memory_used: u64,
    pub memory_limit: u64,
    pub active_textures: usize,
    pub total_textures: usize,
}

#[tauri::command]
pub async fn get_drawing_stats(
    state: State<'_, DrawingState>,
) -> Result<DrawingStats, String> {
    let layers_count = {
        let layers_guard = state.layers.lock().await;
        layers_guard.len()
    };
    
    let (memory_used, memory_limit, active_textures, total_textures) = {
        let engine_guard = state.engine.lock().await;
        let engine = engine_guard.as_ref().ok_or("描画エンジンが初期化されていません")?;
        engine.get_texture_memory_stats().unwrap_or((0, 0, 0, 0))
    };
    
    Ok(DrawingStats {
        layers_count,
        memory_used,
        memory_limit,
        active_textures,
        total_textures,
    })
}

/// 未使用のテクスチャをクリーンアップ
#[tauri::command]
pub async fn cleanup_textures(
    state: State<'_, DrawingState>,
) -> Result<String, String> {
    debug!("[Drawing API] テクスチャクリーンアップ開始");
    
    {
        let mut engine_guard = state.engine.lock().await;
        let engine = engine_guard.as_mut().ok_or("描画エンジンが初期化されていません")?;
        engine.cleanup_unused_textures();
    }
    
    info!("[Drawing API] テクスチャクリーンアップ完了");
    Ok("テクスチャクリーンアップが完了しました".to_string())
}

/// デバッグ用：描画エンジンの詳細状態を取得
#[derive(Serialize)]
pub struct DetailedEngineState {
    pub engine_initialized: bool,
    pub layers: Vec<(String, u32, u32)>, // layer_id, width, height
    pub memory_used: u64,
    pub memory_limit: u64,
    pub active_textures: usize,
    pub total_textures: usize,
}

#[tauri::command]
pub async fn get_detailed_engine_state(
    state: State<'_, DrawingState>,
) -> Result<DetailedEngineState, String> {
    debug!("[Drawing API] 詳細エンジン状態取得開始");
    
    let engine_initialized = {
        let engine_guard = state.engine.lock().await;
        engine_guard.is_some()
    };
    
    let layers = {
        let layers_guard = state.layers.lock().await;
        layers_guard.iter()
            .map(|(k, (w, h))| (k.clone(), *w, *h))
            .collect::<Vec<_>>()
    };
    
    let (memory_used, memory_limit, active_textures, total_textures) = if engine_initialized {
        let engine_guard = state.engine.lock().await;
        let engine = engine_guard.as_ref().unwrap();
        engine.get_texture_memory_stats().unwrap_or((0, 0, 0, 0))
    } else {
        (0, 0, 0, 0)
    };
    
    let state_info = DetailedEngineState {
        engine_initialized,
        layers,
        memory_used,
        memory_limit,
        active_textures,
        total_textures,
    };
    
    debug!("[Drawing API] 詳細エンジン状態: エンジン初期化={}, レイヤー数={}, メモリ使用量={}",
           state_info.engine_initialized, state_info.layers.len(), state_info.memory_used);
    
    Ok(state_info)
}

/// デバッグ用：全レイヤーの詳細情報を取得
#[derive(Serialize)]
pub struct LayerInfo {
    pub layer_id: String,
    pub width: u32,
    pub height: u32,
    pub exists_in_engine: bool,
}

#[tauri::command]
pub async fn get_all_layers_info(
    state: State<'_, DrawingState>,
) -> Result<Vec<LayerInfo>, String> {
    debug!("[Drawing API] 全レイヤー情報取得開始");
    
    let layer_ids = {
        let layers_guard = state.layers.lock().await;
        layers_guard.iter()
            .map(|(k, (w, h))| (k.clone(), *w, *h))
            .collect::<Vec<_>>()
    };
    
    let mut layer_infos = Vec::new();
    
    for (layer_id, width, height) in layer_ids {
        let exists_in_engine = {
            let engine_guard = state.engine.lock().await;
            match engine_guard.as_ref() {
                Some(_engine) => {
                    // エンジンでレイヤーの実際の存在確認は将来の実装で対応
                    // 現時点では状態管理ベースで判定
                    true
                },
                None => false,
            }
        };
        
        layer_infos.push(LayerInfo {
            layer_id: layer_id.clone(),
            width,
            height,
            exists_in_engine,
        });
        
        debug!("[Drawing API] レイヤー情報: {} ({}x{}) エンジン存在={}", 
               layer_id, width, height, exists_in_engine);
    }
    
    info!("[Drawing API] 全レイヤー情報取得完了: {} レイヤー", layer_infos.len());
    Ok(layer_infos)
}

/// デバッグ用：システムメモリ使用量を取得
#[derive(Serialize)]
pub struct SystemMemoryInfo {
    pub process_memory_mb: u64,
    pub available_memory_mb: u64,
    pub texture_memory_mb: u64,
}

#[tauri::command]
pub async fn get_system_memory_info(
    state: State<'_, DrawingState>,
) -> Result<SystemMemoryInfo, String> {
    debug!("[Drawing API] システムメモリ情報取得開始");
    
    // 基本的なメモリ情報取得（プラットフォーム依存部分は簡略化）
    let texture_memory_mb = {
        let engine_guard = state.engine.lock().await;
        match engine_guard.as_ref() {
            Some(engine) => {
                let (used, _limit, _active, _total) = engine.get_texture_memory_stats().unwrap_or((0, 0, 0, 0));
                used / (1024 * 1024) // バイトからMBに変換
            },
            None => 0,
        }
    };
    
    let memory_info = SystemMemoryInfo {
        process_memory_mb: 0, // 将来実装
        available_memory_mb: 0, // 将来実装
        texture_memory_mb,
    };
    
    debug!("[Drawing API] システムメモリ情報: テクスチャメモリ={}MB", memory_info.texture_memory_mb);
    
    Ok(memory_info)
}

/// デバッグ用：詳細状態をログに出力
#[tauri::command]
pub async fn log_detailed_state(
    state: State<'_, DrawingState>,
) -> Result<(), String> {
    debug!("[Drawing API] 詳細状態ログ出力開始");
    
    // 状態管理オブジェクトの詳細ログ出力
    state.log_detailed_state().await;
    
    info!("[Drawing API] 詳細状態ログ出力完了");
    Ok(())
}