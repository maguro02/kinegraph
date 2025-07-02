use log::info;

// コマンド構造定義
pub mod commands;
pub use commands::*;

// ハイブリッドコマンドハンドラー
pub mod hybrid_commands;
pub use hybrid_commands::*;

#[tauri::command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn get_system_info() -> Result<String, String> {
    info!("[API] get_system_info コマンド呼び出し");
    let info = "Kinegraph - Hybrid Drawing Engine".to_string();
    Ok(info)
}

#[tauri::command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn create_drawing_layer(
    state: tauri::State<'_, HybridDrawingState>,
    layer_id: String,
    width: u32,
    height: u32,
) -> Result<CreateLayerResponse, String> {
    info!(
        "[API] create_drawing_layer コマンド呼び出し: layer_id={}, width={}, height={}",
        layer_id, width, height
    );

    let input = UserInput::CreateLayer {
        name: format!("Layer {}", layer_id),
    };

    match process_user_input(state, input).await {
        Ok(commands) => Ok(CreateLayerResponse {
            status: "success".to_string(),
            layer_id,
            commands,
        }),
        Err(e) => Err(e),
    }
}

#[tauri::command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn remove_layer(
    state: tauri::State<'_, HybridDrawingState>,
    layer_id: String,
) -> Result<RemoveLayerResponse, String> {
    info!("[API] remove_layer コマンド呼び出し: layer_id={}", layer_id);

    let input = UserInput::DeleteLayer {
        layer_id: layer_id.clone(),
    };

    match process_user_input(state, input).await {
        Ok(commands) => Ok(RemoveLayerResponse {
            status: "success".to_string(),
            removed_layer_id: layer_id,
            commands,
        }),
        Err(e) => Err(e),
    }
}

#[tauri::command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn initialize_drawing_engine(
    state: tauri::State<'_, HybridDrawingState>,
) -> Result<String, String> {
    info!("[API] initialize_drawing_engine コマンド呼び出し");

    // HybridDrawingStateが正常に初期化されているかを確認
    match get_drawing_state(state).await {
        Ok(_) => Ok("Drawing engine initialized successfully".to_string()),
        Err(e) => Err(format!("Failed to initialize drawing engine: {}", e)),
    }
}
