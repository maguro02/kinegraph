use crate::api::commands::{DrawCommand, UserInput};
use crate::state::{AppState, BlendMode, Layer};
use log::{debug, info};
use serde::Serialize;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

/// ハイブリッド描画エンジンの状態管理
pub struct HybridDrawingState {
    /// アプリケーション全体の状態
    pub app_state: Arc<Mutex<AppState>>,
    /// 現在のツール
    pub current_tool: Arc<Mutex<String>>,
    /// 現在の色
    pub current_color: Arc<Mutex<String>>,
    /// 現在のブラシサイズ
    pub current_brush_size: Arc<Mutex<f32>>,
}

impl HybridDrawingState {
    pub fn new() -> Self {
        let mut app_state = AppState::new();

        // デフォルトレイヤーを作成
        let default_layer_id = format!("layer_{}", uuid::Uuid::new_v4());
        let default_layer = Layer {
            id: default_layer_id.clone(),
            name: "Layer 1".to_string(),
            visible: true,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
        };
        app_state.layers.push(default_layer);
        app_state.active_layer_id = Some(default_layer_id);

        Self {
            app_state: Arc::new(Mutex::new(app_state)),
            current_tool: Arc::new(Mutex::new("pen".to_string())),
            current_color: Arc::new(Mutex::new("#000000".to_string())),
            current_brush_size: Arc::new(Mutex::new(2.0)),
        }
    }
}

/// ユーザー入力を処理し、描画コマンドを返す
#[tauri::command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn process_user_input(
    state: State<'_, HybridDrawingState>,
    input: UserInput,
) -> Result<Vec<DrawCommand>, String> {
    debug!("Processing user input: {:?}", input);

    match input {
        UserInput::DrawStroke {
            points,
            color,
            width,
            layer_id,
        } => {
            // ストローク描画の処理
            let app_state = state.app_state.lock().await;

            // レイヤーが存在するか確認
            if !app_state.layers.iter().any(|l| l.id == layer_id) {
                return Err("Layer not found".to_string());
            }

            // ストロークを状態に追加（将来的にはUndo/Redo用）
            // TODO: ストローク履歴の管理

            // 描画コマンドを生成
            let commands = vec![DrawCommand::DrawPath {
                points,
                color,
                width,
                layer_id,
            }];

            Ok(commands)
        }

        UserInput::ChangeTool { tool_id } => {
            // ツール変更
            let mut current_tool = state.current_tool.lock().await;
            *current_tool = tool_id;
            info!("Tool changed to: {}", *current_tool);
            Ok(vec![])
        }

        UserInput::CreateLayer { name } => {
            // レイヤー作成
            let mut app_state = state.app_state.lock().await;
            let layer_id = format!("layer_{}", uuid::Uuid::new_v4());
            let new_layer = Layer {
                id: layer_id.clone(),
                name,
                visible: true,
                opacity: 1.0,
                blend_mode: BlendMode::Normal,
            };

            let index = app_state.layers.len();
            app_state.layers.push(new_layer);

            // アクティブレイヤーが未設定の場合は設定
            if app_state.active_layer_id.is_none() {
                app_state.active_layer_id = Some(layer_id.clone());
            }

            Ok(vec![DrawCommand::AddLayer { layer_id, index }])
        }

        UserInput::DeleteLayer { layer_id } => {
            // レイヤー削除
            let mut app_state = state.app_state.lock().await;
            app_state.layers.retain(|l| l.id != layer_id);

            Ok(vec![DrawCommand::RemoveLayer { layer_id }])
        }

        UserInput::ReorderLayer {
            layer_id,
            new_index,
        } => {
            // レイヤー順序変更
            let mut app_state = state.app_state.lock().await;

            // 現在のインデックスを取得
            let current_index = app_state.layers.iter().position(|l| l.id == layer_id);

            if let Some(current_idx) = current_index {
                if new_index < app_state.layers.len() {
                    let layer = app_state.layers.remove(current_idx);
                    app_state.layers.insert(new_index, layer);

                    // 新しい順序でレイヤーIDリストを作成
                    let layer_ids: Vec<String> =
                        app_state.layers.iter().map(|l| l.id.clone()).collect();

                    return Ok(vec![DrawCommand::ReorderLayers { layer_ids }]);
                }
            }

            Err("Invalid layer index".to_string())
        }

        UserInput::ChangeLayerOpacity { layer_id, opacity } => {
            // レイヤー不透明度変更
            let mut app_state = state.app_state.lock().await;

            if let Some(layer) = app_state.layers.iter_mut().find(|l| l.id == layer_id) {
                layer.opacity = opacity;

                return Ok(vec![DrawCommand::UpdateLayerProperties {
                    layer_id,
                    opacity,
                    blend_mode: layer.blend_mode.to_string(),
                    visible: layer.visible,
                }]);
            }

            Err("Layer not found".to_string())
        }

        UserInput::ChangeLayerBlendMode {
            layer_id,
            blend_mode,
        } => {
            // レイヤーブレンドモード変更
            let mut app_state = state.app_state.lock().await;

            if let Some(layer) = app_state.layers.iter_mut().find(|l| l.id == layer_id) {
                // ブレンドモードを文字列から変換
                layer.blend_mode = match blend_mode.as_str() {
                    "multiply" => BlendMode::Multiply,
                    "screen" => BlendMode::Screen,
                    "overlay" => BlendMode::Overlay,
                    _ => BlendMode::Normal,
                };

                return Ok(vec![DrawCommand::UpdateLayerProperties {
                    layer_id,
                    opacity: layer.opacity,
                    blend_mode,
                    visible: layer.visible,
                }]);
            }

            Err("Layer not found".to_string())
        }

        UserInput::Fill {
            point,
            color,
            layer_id,
        } => {
            // 塗りつぶし処理
            // TODO: 実際の塗りつぶしアルゴリズムの実装
            info!(
                "Fill at {:?} with color {} on layer {}",
                point, color, layer_id
            );

            // 仮実装：塗りつぶし領域を計算してピクセルデータを生成
            // 実際には、現在のレイヤーのピクセルデータを読み込み、
            // 塗りつぶしアルゴリズムを実行し、変更された領域のみを更新する

            Ok(vec![])
        }

        UserInput::CreateSelection {
            selection_type,
            points,
        } => {
            // 選択範囲作成
            Ok(vec![DrawCommand::ShowSelection {
                selection_type,
                points,
            }])
        }

        UserInput::TransformSelection { transform } => {
            // 選択範囲の変形
            // TODO: 現在のアクティブレイヤーと選択範囲を取得
            let app_state = state.app_state.lock().await;

            if let Some(active_layer_id) = &app_state.active_layer_id {
                return Ok(vec![DrawCommand::ApplyTransform {
                    layer_id: active_layer_id.clone(),
                    transform,
                }]);
            }

            Err("No active layer".to_string())
        }

        UserInput::Undo => {
            // Undo処理
            // TODO: 操作履歴の実装
            info!("Undo requested");
            Ok(vec![])
        }

        UserInput::Redo => {
            // Redo処理
            // TODO: 操作履歴の実装
            info!("Redo requested");
            Ok(vec![])
        }
    }
}

/// 現在の描画状態を取得
#[tauri::command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn get_drawing_state(
    state: State<'_, HybridDrawingState>,
) -> Result<DrawingStateInfo, String> {
    let app_state = state.app_state.lock().await;
    let current_tool = state.current_tool.lock().await;
    let current_color = state.current_color.lock().await;
    let current_brush_size = state.current_brush_size.lock().await;

    Ok(DrawingStateInfo {
        layers: app_state
            .layers
            .iter()
            .map(|l| HybridLayerInfo {
                id: l.id.clone(),
                name: l.name.clone(),
                visible: l.visible,
                opacity: l.opacity,
                blend_mode: l.blend_mode.to_string(),
            })
            .collect(),
        active_layer_id: app_state.active_layer_id.clone(),
        current_tool: current_tool.clone(),
        current_color: current_color.clone(),
        current_brush_size: *current_brush_size,
    })
}

#[derive(Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct DrawingStateInfo {
    pub layers: Vec<HybridLayerInfo>,
    pub active_layer_id: Option<String>,
    pub current_tool: String,
    pub current_color: String,
    pub current_brush_size: f32,
}

#[derive(Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct HybridLayerInfo {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub blend_mode: String,
}
