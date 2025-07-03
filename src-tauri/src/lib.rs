// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

pub mod animation {
    include!("../animation/mod.rs");
}

pub mod api {
    include!("../api/mod.rs");
}

pub mod state {
    include!("../state/mod.rs");
}

pub mod drawing_engine;
pub mod ipc;

#[cfg(feature = "specta")]
pub mod tauri_bindings;

use log::{debug, error, info};
use std::sync::Arc;

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

    // ハイブリッド描画エンジンの状態管理
    debug!("[KINEGRAPH] HybridDrawingState 初期化中...");
    let hybrid_drawing_state = api::HybridDrawingState::new();
    debug!("[KINEGRAPH] HybridDrawingState 初期化完了");
    
    // 新しい描画エンジンの初期化
    debug!("[KINEGRAPH] DrawingEngine 初期化中...");
    let drawing_engine = tauri::async_runtime::block_on(async {
        match drawing_engine::DrawingEngine::new().await {
            Ok(engine) => {
                debug!("[KINEGRAPH] DrawingEngine 初期化完了");
                Arc::new(engine)
            }
            Err(e) => {
                error!("[KINEGRAPH] DrawingEngine 初期化失敗: {}", e);
                panic!("DrawingEngine の初期化に失敗しました: {}", e);
            }
        }
    });
    
    // レンダーキャッシュの初期化
    debug!("[KINEGRAPH] RenderCache 初期化中...");
    let render_cache = Arc::new(ipc::RenderCache::new());
    debug!("[KINEGRAPH] RenderCache 初期化完了");

    // Tauri状態管理に登録
    debug!("[KINEGRAPH] Tauri Builder 初期化中...");
    let builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());

    debug!("[KINEGRAPH] HybridDrawingState を Tauri 状態管理に登録中...");
    let builder = builder.manage(hybrid_drawing_state);
    debug!("[KINEGRAPH] HybridDrawingState 状態管理登録完了");
    
    debug!("[KINEGRAPH] DrawingEngine を Tauri 状態管理に登録中...");
    let builder = builder.manage(drawing_engine);
    debug!("[KINEGRAPH] DrawingEngine 状態管理登録完了");
    
    debug!("[KINEGRAPH] RenderCache を Tauri 状態管理に登録中...");
    let builder = builder.manage(render_cache);
    debug!("[KINEGRAPH] RenderCache 状態管理登録完了");

    debug!("[KINEGRAPH] Tauri invoke_handler 登録中...");

    let builder = builder.invoke_handler(tauri::generate_handler![
        // システム情報
        api::get_system_info,
        // ハイブリッド描画API
        api::process_user_input,
        api::get_drawing_state,
        api::initialize_drawing_engine,
        // レイヤー管理API
        api::create_drawing_layer,
        api::remove_layer,
        // 新しい描画エンジンAPI
        ipc::init_canvas,
        ipc::draw_command,
        ipc::get_render_result,
        ipc::resize_canvas,
        ipc::get_compressed_render_result,
        ipc::get_diff_render_result,
        ipc::clear_render_cache
    ]);

    debug!("[KINEGRAPH] invoke_handler 登録完了");

    info!("[KINEGRAPH] Tauri アプリケーション実行開始");
    match builder.run(tauri::generate_context!()) {
        Ok(_) => info!("[KINEGRAPH] アプリケーション正常終了"),
        Err(e) => {
            error!("[KINEGRAPH] アプリケーション実行エラー: {e}");
            panic!("Tauri アプリケーション実行に失敗しました: {e}");
        }
    }
}
