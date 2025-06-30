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

#[cfg(feature = "specta")]
pub mod tauri_bindings;

use drawing_engine::DrawingEngine;
use api::drawing::DrawingState;
use log::{info, error, debug};

// greet function commented out due to macro conflict

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ログレベルの初期化（デバッグ用に詳細レベル設定）
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .format_timestamp_secs()
        .format_module_path(true)
        .init();
    
    info!("[KINEGRAPH] アプリケーション起動開始");
    
    // DrawingEngine の初期化（既存API用）
    debug!("[KINEGRAPH] DrawingEngine インスタンス作成中...");
    let drawing_engine = DrawingEngine::new();
    debug!("[KINEGRAPH] DrawingEngine インスタンス作成完了");
    
    // Arc<Mutex>でラップ（既存API用）
    debug!("[KINEGRAPH] DrawingEngine を Arc<Mutex> でラップ中...");
    let drawing_engine = std::sync::Arc::new(tokio::sync::Mutex::new(drawing_engine));
    debug!("[KINEGRAPH] DrawingEngine ラップ完了");
    
    // 新しい描画API用の状態管理
    debug!("[KINEGRAPH] DrawingState 初期化中...");
    let drawing_state = DrawingState::new();
    debug!("[KINEGRAPH] DrawingState 初期化完了");
    
    // Tauri状態管理に登録
    debug!("[KINEGRAPH] Tauri Builder 初期化中...");
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init());
    
    debug!("[KINEGRAPH] DrawingEngine を Tauri 状態管理に登録中...");
    let builder = builder.manage(drawing_engine.clone());
    debug!("[KINEGRAPH] DrawingEngine 状態管理登録完了");
    
    debug!("[KINEGRAPH] DrawingState を Tauri 状態管理に登録中...");
    let builder = builder.manage(drawing_state);
    debug!("[KINEGRAPH] DrawingState 状態管理登録完了");
    
    debug!("[KINEGRAPH] Tauri invoke_handler 登録中...");
    
    let builder = builder.invoke_handler(tauri::generate_handler![
        // 既存のプロジェクトAPI
        api::create_project,
        api::get_system_info,
        api::create_layer,
        api::draw_line,
        api::draw_stroke,
        api::get_layer_data,
        
        // 新しい描画API
        api::initialize_drawing_engine,
        api::create_drawing_layer,
        api::draw_line_on_layer,
        api::draw_stroke_on_layer,
        api::get_layer_image_data,
        api::clear_layer,
        api::remove_layer,
        api::get_drawing_stats,
        api::cleanup_textures,
        
        // デバッグAPI
        api::get_detailed_engine_state,
        api::get_all_layers_info,
        api::get_system_memory_info,
        api::log_detailed_state
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
