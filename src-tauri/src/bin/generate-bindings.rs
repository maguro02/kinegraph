fn main() {
    #[cfg(feature = "specta")]
    {
        if let Err(e) = kinegraph_lib::tauri_bindings::generate_bindings() {
            eprintln!("Failed to generate bindings: {}", e);
            std::process::exit(1);
        }
        println!("Bindings generated successfully!");
    }

    #[cfg(not(feature = "specta"))]
    {
        println!("specta feature is not enabled. Run with: cargo run --bin generate-bindings --features specta");
    }
}
