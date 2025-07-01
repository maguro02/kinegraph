use log::info;

// コマンド構造定義
pub mod commands;
pub use commands::*;

// ハイブリッドコマンドハンドラー
pub mod hybrid_commands;
pub use hybrid_commands::*;


#[tauri::command]
pub async fn get_system_info() -> Result<String, String> {
    info!("[API] get_system_info コマンド呼び出し");
    let info = "Kinegraph - Hybrid Drawing Engine".to_string();
    Ok(info)
}

