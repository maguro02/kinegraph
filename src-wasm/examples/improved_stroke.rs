use wasm_bindgen::prelude::*;
use web_sys::console;
use js_sys::Float32Array;

// Improved stroke handling with direct Float32Array support
#[wasm_bindgen]
pub struct ImprovedDrawingContext {
    width: u32,
    height: u32,
    stroke_data: Vec<f32>,
}

#[wasm_bindgen]
impl ImprovedDrawingContext {
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32) -> Result<ImprovedDrawingContext, JsValue> {
        // Validate dimensions early
        if width == 0 || height == 0 || width > 8192 || height > 8192 {
            return Err(JsValue::from_str("Invalid canvas dimensions"));
        }
        
        Ok(ImprovedDrawingContext {
            width,
            height,
            stroke_data: Vec::new(),
        })
    }
    
    // Direct Float32Array handling - no intermediate conversion
    pub fn add_stroke_points(&mut self, points: Float32Array) -> Result<(), JsValue> {
        console::log_1(&format!("Receiving {} points", points.length()).into());
        
        // Validate input
        if points.length() == 0 || points.length() % 2 != 0 {
            return Err(JsValue::from_str("Invalid points array: must have even number of values"));
        }
        
        // Use to_vec() for efficient conversion
        let points_vec = points.to_vec();
        
        // Validate point values
        for (i, &val) in points_vec.iter().enumerate() {
            if val.is_nan() || val.is_infinite() {
                return Err(JsValue::from_str(&format!("Invalid point value at index {}", i)));
            }
        }
        
        // Store points for processing
        self.stroke_data.extend_from_slice(&points_vec);
        
        Ok(())
    }
    
    // Return Float32Array directly to JavaScript
    pub fn get_stroke_data(&self) -> Float32Array {
        Float32Array::from(&self.stroke_data[..])
    }
    
    // Process stroke with error handling
    pub fn process_stroke(&mut self, color: &[f32], width: f32) -> Result<Float32Array, JsValue> {
        if self.stroke_data.len() < 4 {
            return Err(JsValue::from_str("Need at least 2 points for a stroke"));
        }
        
        if width <= 0.0 || width > 100.0 {
            return Err(JsValue::from_str("Invalid stroke width"));
        }
        
        // Simulate processing - in real implementation, this would generate vertices
        let mut processed_data = Vec::new();
        
        for chunk in self.stroke_data.chunks(2) {
            let x = chunk[0];
            let y = chunk[1];
            
            // Add processed vertices (simplified example)
            processed_data.push(x - width / 2.0);
            processed_data.push(y);
            processed_data.push(x + width / 2.0);
            processed_data.push(y);
        }
        
        // Clear stroke data after processing
        self.stroke_data.clear();
        
        Ok(Float32Array::from(&processed_data[..]))
    }
    
    // Safe memory view creation for SharedArrayBuffer
    pub fn create_shared_view(&self, buffer: &js_sys::SharedArrayBuffer, offset: u32) -> Result<Float32Array, JsValue> {
        let byte_offset = offset * 4; // Float32 is 4 bytes
        let byte_length = (self.width * self.height * 4 * 4) as u32; // RGBA * 4 bytes per float
        
        // Validate buffer size
        if byte_offset + byte_length > buffer.byte_length() {
            return Err(JsValue::from_str("SharedArrayBuffer too small"));
        }
        
        Ok(Float32Array::new_with_byte_offset_and_length(
            buffer,
            byte_offset,
            byte_length / 4
        ))
    }
}

// WebWorker message handling with proper error propagation
#[wasm_bindgen]
pub struct ImprovedWorkerHandler {
    context: Option<ImprovedDrawingContext>,
}

#[wasm_bindgen]
impl ImprovedWorkerHandler {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        ImprovedWorkerHandler { context: None }
    }
    
    pub fn init_context(&mut self, width: u32, height: u32) -> Result<(), JsValue> {
        match ImprovedDrawingContext::new(width, height) {
            Ok(ctx) => {
                self.context = Some(ctx);
                Ok(())
            }
            Err(e) => Err(e)
        }
    }
    
    pub fn handle_stroke(&mut self, points: Float32Array, color: Float32Array, width: f32) -> Result<Float32Array, JsValue> {
        let ctx = self.context.as_mut()
            .ok_or_else(|| JsValue::from_str("Context not initialized"))?;
        
        // Validate color array
        if color.length() < 4 {
            return Err(JsValue::from_str("Color must have 4 components (RGBA)"));
        }
        
        let color_vec = color.to_vec();
        ctx.add_stroke_points(points)?;
        ctx.process_stroke(&color_vec, width)
    }
}

// Utility functions for safe WebGPU initialization
#[wasm_bindgen]
pub async fn safe_init_webgpu() -> Result<JsValue, JsValue> {
    // Check for WebGPU support first
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window object"))?;
    let navigator = window.navigator();
    
    let gpu = js_sys::Reflect::get(&navigator, &JsValue::from_str("gpu"))
        .map_err(|_| JsValue::from_str("Failed to access navigator.gpu"))?;
    
    if gpu.is_undefined() {
        return Err(JsValue::from_str("WebGPU not supported"));
    }
    
    // Return success status
    Ok(JsValue::from_str("WebGPU available"))
}

// Example of safe memory management pattern
#[wasm_bindgen]
pub struct MemoryPool {
    buffers: Vec<Vec<f32>>,
    free_indices: Vec<usize>,
}

#[wasm_bindgen]
impl MemoryPool {
    #[wasm_bindgen(constructor)]
    pub fn new(pool_size: usize, buffer_size: usize) -> Self {
        let mut buffers = Vec::with_capacity(pool_size);
        let mut free_indices = Vec::with_capacity(pool_size);
        
        for i in 0..pool_size {
            buffers.push(vec![0.0; buffer_size]);
            free_indices.push(i);
        }
        
        MemoryPool { buffers, free_indices }
    }
    
    pub fn acquire(&mut self) -> Result<usize, JsValue> {
        self.free_indices.pop()
            .ok_or_else(|| JsValue::from_str("No free buffers available"))
    }
    
    pub fn release(&mut self, index: usize) -> Result<(), JsValue> {
        if index >= self.buffers.len() {
            return Err(JsValue::from_str("Invalid buffer index"));
        }
        
        // Clear buffer before returning to pool
        self.buffers[index].fill(0.0);
        self.free_indices.push(index);
        
        Ok(())
    }
    
    pub fn get_buffer(&mut self, index: usize) -> Result<Float32Array, JsValue> {
        self.buffers.get(index)
            .map(|buf| Float32Array::from(&buf[..]))
            .ok_or_else(|| JsValue::from_str("Invalid buffer index"))
    }
}