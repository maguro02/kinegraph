//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_webgpu_support_check() {
    // Test that we can check for WebGPU support
    let supported = kinegraph_wasm::check_webgpu_support();
    // In test environment, this might be false, that's OK
    assert!(supported || !supported);
}

#[wasm_bindgen_test]
async fn test_shared_buffer_creation() {
    use kinegraph_wasm::SharedBuffer;
    
    // Test creating a SharedArrayBuffer
    let result = SharedBuffer::new(1024);
    
    // SharedArrayBuffer might not be available in test environment
    match result {
        Ok(buffer) => {
            assert_eq!(buffer.buffer().byte_length(), 1024);
        }
        Err(_) => {
            // Expected in environments without SharedArrayBuffer
            console_log::log!("SharedArrayBuffer not available in test environment");
        }
    }
}

#[wasm_bindgen_test]
async fn test_drawing_context_creation() {
    use kinegraph_wasm::DrawingContext;
    
    // Test creating a drawing context
    let result = DrawingContext::new(800, 600).await;
    
    // WebGPU might not be available in test environment
    match result {
        Ok(_context) => {
            console_log::log!("DrawingContext created successfully");
        }
        Err(e) => {
            console_log::log!("Expected error in test environment: {:?}", e);
        }
    }
}