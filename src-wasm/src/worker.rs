use wasm_bindgen::prelude::*;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent, OffscreenCanvas};
use crate::{DrawingContext, SharedBuffer};

#[wasm_bindgen]
pub struct WorkerContext {
    drawing_context: Option<DrawingContext>,
    shared_buffer: Option<SharedBuffer>,
}

impl WorkerContext {
    pub fn new() -> Self {
        Self {
            drawing_context: None,
            shared_buffer: None,
        }
    }
}

#[wasm_bindgen]
impl WorkerContext {
    
    pub async fn init(&mut self, canvas: OffscreenCanvas) -> Result<(), JsValue> {
        let width = canvas.width();
        let height = canvas.height();
        
        // Initialize drawing context with the transferred canvas
        let context = DrawingContext::new(width, height).await?;
        
        // Create shared buffer for pixel data transfer
        let buffer_size = width * height * 4; // RGBA
        let shared_buffer = SharedBuffer::new(buffer_size)?;
        
        self.drawing_context = Some(context);
        self.shared_buffer = Some(shared_buffer);
        
        Ok(())
    }
    
    pub fn process_draw_command(&mut self, command: JsValue) -> Result<(), JsValue> {
        // Check context exists before processing
        let context = self.drawing_context.as_mut()
            .ok_or_else(|| JsValue::from_str("Drawing context not initialized"))?;
        
        // Parse draw command from main thread with error handling
        let command_obj: DrawCommand = serde_wasm_bindgen::from_value(command)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse command: {:?}", e)))?;
        
        match command_obj {
            DrawCommand::Stroke { points, color, width } => {
                // Additional validation before passing to context
                if points.is_empty() {
                    return Err(JsValue::from_str("Empty points array in stroke command"));
                }
                if color.len() < 4 {
                    return Err(JsValue::from_str("Invalid color array in stroke command"));
                }
                context.draw_stroke(&points, &color, width)?;
            }
            DrawCommand::Clear { color } => {
                context.clear(&color)?;
            }
            DrawCommand::GetPixels => {
                if let Some(ref buffer) = self.shared_buffer {
                    let pixels = context.get_pixels()?;
                    buffer.write_pixels(0, &pixels)?;
                }
            }
            DrawCommand::StrokeTypedArray { points, color, width } => {
                // Get the actual Float32Array data
                let points_array = points.ok_or_else(|| JsValue::from_str("Points array is required"))?;
                let points_vec: Vec<f32> = points_array.to_vec();
                
                let color_vec: Vec<f32> = color.map(|c| c.to_vec()).unwrap_or_else(|| vec![0.0, 0.0, 0.0, 1.0]);
                
                // Additional validation before passing to context
                if points_vec.is_empty() {
                    return Err(JsValue::from_str("Empty points array in stroke command"));
                }
                if color_vec.len() < 4 {
                    return Err(JsValue::from_str("Invalid color array in stroke command"));
                }
                
                context.draw_stroke(&points_vec, &color_vec, width)?;
            }
        }
        
        Ok(())
    }
}

#[derive(serde::Deserialize)]
#[serde(tag = "type")]
enum DrawCommand {
    Stroke {
        points: Vec<f32>,
        color: Vec<f32>,
        width: f32,
    },
    Clear {
        color: Vec<f32>,
    },
    GetPixels,
    // New command for typed array handling
    StrokeTypedArray {
        #[serde(skip)]
        points: Option<js_sys::Float32Array>,
        #[serde(skip)]
        color: Option<js_sys::Float32Array>,
        width: f32,
    },
}

// Worker message handler
#[wasm_bindgen]
pub fn setup_worker(scope: &DedicatedWorkerGlobalScope) -> Result<(), JsValue> {
    let context = std::rc::Rc::new(std::cell::RefCell::new(WorkerContext::new()));
    let context_clone = context.clone();
    
    let onmessage_callback = Closure::wrap(Box::new(move |event: MessageEvent| {
        let data = event.data();
        
        // Handle different message types
        // Try to parse the message type first
        if let Ok(msg_type) = js_sys::Reflect::get(&data, &JsValue::from_str("type")) {
            if let Some(msg_type_str) = msg_type.as_string() {
                match msg_type_str.as_str() {
                    "Init" => {
                        if let Ok(canvas) = js_sys::Reflect::get(&data, &JsValue::from_str("canvas")) {
                            if let Ok(canvas) = canvas.dyn_into::<OffscreenCanvas>() {
                                let context = context_clone.clone();
                                wasm_bindgen_futures::spawn_local(async move {
                                    if let Err(e) = context.borrow_mut().init(canvas).await {
                                        web_sys::console::error_1(&e);
                                    }
                                });
                            }
                        }
                    }
                    "DrawCommand" => {
                        if let Ok(command) = js_sys::Reflect::get(&data, &JsValue::from_str("command")) {
                            if let Err(e) = context_clone.borrow_mut().process_draw_command(command) {
                                web_sys::console::error_1(&e);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }) as Box<dyn FnMut(MessageEvent)>);
    
    scope.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();
    
    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(tag = "type")]
enum WorkerMessage {
    Init {
        #[serde(skip)]
        canvas: Option<OffscreenCanvas>,
    },
    DrawCommand {
        #[serde(skip)]
        command: Option<JsValue>,
    },
}