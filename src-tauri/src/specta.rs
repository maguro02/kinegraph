use tauri_specta::ts;
use tauri_specta::{collect_commands, Builder};

// Re-export API commands
pub use crate::api::{
    create_drawing_layer, get_drawing_state, get_system_info, initialize_drawing_engine,
    process_user_input, remove_layer,
};

/// Generate TypeScript bindings for all Tauri commands
pub fn export_typescript_bindings() -> Result<(), Box<dyn std::error::Error>> {
    let builder = Builder::<tauri::Wry>::new();

    // 実際に存在するコマンドのみ
    let builder = builder.command(get_system_info);
    let builder = builder.command(process_user_input);
    let builder = builder.command(get_drawing_state);
    let builder = builder.command(initialize_drawing_engine);
    let builder = builder.command(create_drawing_layer);
    let builder = builder.command(remove_layer);

    let (invoke, _events) = builder.build()?;

    // TypeScript bindings をエクスポート
    invoke.export(ts::Typescript::default(), "../src/lib/bindings.ts")?;

    println!("TypeScript bindings exported successfully!");
    Ok(())
}

/// Get all commands for Tauri invoke handler
pub fn get_all_commands() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool {
    tauri::generate_handler![
        // 実際に存在するコマンドのみ
        get_system_info,
        process_user_input,
        get_drawing_state,
        initialize_drawing_engine,
        create_drawing_layer,
        remove_layer
    ]
}
