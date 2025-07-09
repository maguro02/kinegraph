use tauri_specta::ts;
use tauri_specta::{collect_commands, Builder};

// Re-export API commands
pub use crate::api::{
    create_drawing_layer, get_drawing_state, get_system_info, initialize_drawing_engine,
    process_user_input, remove_layer,
};

// Re-export IPC commands (including binary handlers)
pub use crate::ipc::{
    init_canvas, draw_command, get_render_result, resize_canvas,
    get_compressed_render_result, get_diff_render_result, clear_render_cache,
    // Binary handlers
    draw_binary, draw_batch_binary, get_canvas_data_binary, get_render_result_binary,
    get_canvas_data_binary_stream, get_canvas_data_chunked, get_canvas_data_perf_test,
};

/// Generate TypeScript bindings for all Tauri commands
pub fn export_typescript_bindings() -> Result<(), Box<dyn std::error::Error>> {
    let builder = Builder::<tauri::Wry>::new();

    // API commands
    let builder = builder.command(get_system_info);
    let builder = builder.command(process_user_input);
    let builder = builder.command(get_drawing_state);
    let builder = builder.command(initialize_drawing_engine);
    let builder = builder.command(create_drawing_layer);
    let builder = builder.command(remove_layer);
    
    // IPC commands
    let builder = builder.command(init_canvas);
    let builder = builder.command(draw_command);
    let builder = builder.command(get_render_result);
    let builder = builder.command(resize_canvas);
    let builder = builder.command(get_compressed_render_result);
    let builder = builder.command(get_diff_render_result);
    let builder = builder.command(clear_render_cache);
    
    // Binary handlers
    let builder = builder.command(draw_binary);
    let builder = builder.command(draw_batch_binary);
    let builder = builder.command(get_canvas_data_binary);
    let builder = builder.command(get_render_result_binary);
    let builder = builder.command(get_canvas_data_binary_stream);
    let builder = builder.command(get_canvas_data_chunked);
    let builder = builder.command(get_canvas_data_perf_test);

    let (invoke, _events) = builder.build()?;

    // TypeScript bindings をエクスポート
    invoke.export(ts::Typescript::default(), "../src/lib/bindings.ts")?;

    println!("TypeScript bindings exported successfully!");
    Ok(())
}

/// Get all commands for Tauri invoke handler
pub fn get_all_commands() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool {
    tauri::generate_handler![
        // API commands
        get_system_info,
        process_user_input,
        get_drawing_state,
        initialize_drawing_engine,
        create_drawing_layer,
        remove_layer,
        // IPC commands
        init_canvas,
        draw_command,
        get_render_result,
        resize_canvas,
        get_compressed_render_result,
        get_diff_render_result,
        clear_render_cache,
        // Binary handlers
        draw_binary,
        draw_batch_binary,
        get_canvas_data_binary,
        get_render_result_binary,
        get_canvas_data_binary_stream,
        get_canvas_data_chunked,
        get_canvas_data_perf_test
    ]
}
