// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

pub mod animation {
    include!("../animation/mod.rs");
}

pub mod api {
    include!("../api/mod.rs");
}

pub mod drawing_engine {
    include!("../drawing_engine/mod.rs");
}

use drawing_engine::DrawingEngine;
use log::{info, error, debug};

// greet function commented out due to macro conflict

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ログレベルの初期化
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();
    
    info!("[KINEGRAPH] アプリケーション起動開始");
    
    // DrawingEngine の初期化
    debug!("[KINEGRAPH] DrawingEngine インスタンス作成中...");
    let drawing_engine = DrawingEngine::new();
    debug!("[KINEGRAPH] DrawingEngine インスタンス作成完了");
    
    // Arc<Mutex>でラップ
    debug!("[KINEGRAPH] DrawingEngine を Arc<Mutex> でラップ中...");
    let drawing_engine = std::sync::Arc::new(tokio::sync::Mutex::new(drawing_engine));
    debug!("[KINEGRAPH] DrawingEngine ラップ完了");
    
    // Tauri状態管理に登録
    debug!("[KINEGRAPH] Tauri Builder 初期化中...");
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init());
    
    debug!("[KINEGRAPH] DrawingEngine を Tauri 状態管理に登録中...");
    let builder = builder.manage(drawing_engine.clone());
    debug!("[KINEGRAPH] DrawingEngine 状態管理登録完了");
    
    debug!("[KINEGRAPH] Tauri invoke_handler 登録中...");
    let builder = builder.invoke_handler(tauri::generate_handler![
        api::create_project,
        api::get_system_info
    ]);
    debug!("[KINEGRAPH] invoke_handler 登録完了");
    
    info!("[KINEGRAPH] Tauri アプリケーション実行開始");
    match builder.run(tauri::generate_context!()) {
        Ok(_) => info!("[KINEGRAPH] アプリケーション正常終了"),
        Err(e) => {
            error!("[KINEGRAPH] アプリケーション実行エラー: {}", e);
            panic!("Tauri アプリケーション実行に失敗しました: {}", e);
        }
    }
}
