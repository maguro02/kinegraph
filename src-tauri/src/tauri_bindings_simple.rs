use tauri_specta::collect_commands;

pub fn generate_bindings() {
    tauri_specta::ts::export(
        collect_commands![
            // 既存のプロジェクトAPI
            crate::api::create_project,
            crate::api::get_system_info,
            crate::api::create_layer,
            crate::api::draw_line,
            crate::api::draw_stroke,
            crate::api::get_layer_data,
            
            // 新しい描画API
            crate::api::initialize_drawing_engine,
            crate::api::create_drawing_layer,
            crate::api::draw_line_on_layer,
            crate::api::draw_stroke_on_layer,
            crate::api::get_layer_image_data,
            crate::api::clear_layer,
            crate::api::remove_layer,
            crate::api::get_drawing_stats,
            crate::api::cleanup_textures,
            
            // デバッグAPI
            crate::api::get_detailed_engine_state,
            crate::api::get_all_layers_info,
            crate::api::get_system_memory_info,
            crate::api::log_detailed_state
        ],
        "../src/lib/bindings.ts"
    ).expect("Failed to export typescript bindings");
}