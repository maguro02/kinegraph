use specta_typescript::Typescript;
use tauri_specta::{collect_commands, Builder};

pub fn generate_bindings() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Typescript::default();
    config.bigint = specta_typescript::BigIntExportBehavior::Number;

    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            crate::api::get_system_info,
            crate::api::process_user_input,
            crate::api::get_drawing_state,
            crate::api::initialize_drawing_engine,
            crate::api::create_drawing_layer,
            crate::api::remove_layer,
        ])
        .export(config, "../src/lib/bindings.ts")?;
    Ok(())
}
