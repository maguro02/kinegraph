fn main() {
    #[cfg(feature = "specta")]
    {
        kinegraph_lib::tauri_bindings::generate_bindings();
    }
    
    #[cfg(not(feature = "specta"))]
    {
        println!("specta feature is not enabled. Run with: cargo run --bin generate-bindings --features specta");
    }
}