use tauri_specta::{collect_commands, Builder};
use tauri_specta::ts;

// Re-export API commands
pub use crate::api::{
    create_project, get_system_info, create_layer, draw_line, draw_stroke, get_layer_data,
    initialize_drawing_engine, create_drawing_layer, draw_line_on_layer, draw_stroke_on_layer,
    get_layer_image_data, clear_layer, remove_layer, get_drawing_stats, cleanup_textures,
    get_detailed_engine_state, get_all_layers_info, get_system_memory_info, log_detailed_state
};

/// Generate TypeScript bindings for all Tauri commands
pub fn export_typescript_bindings() -> Result<(), Box<dyn std::error::Error>> {
    let builder = Builder::<tauri::Wry>::new();
    
    // 既存のプロジェクトAPI
    let builder = builder.command(create_project);
    let builder = builder.command(get_system_info);
    let builder = builder.command(create_layer);
    let builder = builder.command(draw_line);
    let builder = builder.command(draw_stroke);
    let builder = builder.command(get_layer_data);
    
    // 新しい描画API
    let builder = builder.command(initialize_drawing_engine);
    let builder = builder.command(create_drawing_layer);
    let builder = builder.command(draw_line_on_layer);
    let builder = builder.command(draw_stroke_on_layer);
    let builder = builder.command(get_layer_image_data);
    let builder = builder.command(clear_layer);
    let builder = builder.command(remove_layer);
    let builder = builder.command(get_drawing_stats);
    let builder = builder.command(cleanup_textures);
    
    // デバッグAPI
    let builder = builder.command(get_detailed_engine_state);
    let builder = builder.command(get_all_layers_info);
    let builder = builder.command(get_system_memory_info);
    let builder = builder.command(log_detailed_state);
    
    let (invoke, _events) = builder.build()?;
    
    // TypeScript bindings をエクスポート
    invoke.export(ts::Typescript::default(), "../src/lib/bindings.ts")?;
    
    println!("TypeScript bindings exported successfully!");
    Ok(())
}

/// Get all commands for Tauri invoke handler
pub fn get_all_commands() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool {
    tauri::generate_handler![
        // 既存のプロジェクトAPI
        create_project,
        get_system_info,
        create_layer,
        draw_line,
        draw_stroke,
        get_layer_data,
        
        // 新しい描画API
        initialize_drawing_engine,
        create_drawing_layer,
        draw_line_on_layer,
        draw_stroke_on_layer,
        get_layer_image_data,
        clear_layer,
        remove_layer,
        get_drawing_stats,
        cleanup_textures,
        
        // デバッグAPI
        get_detailed_engine_state,
        get_all_layers_info,
        get_system_memory_info,
        log_detailed_state
    ]
}