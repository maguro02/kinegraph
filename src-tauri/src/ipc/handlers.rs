use tauri::{command, State};
use crate::drawing_engine::{DrawingEngine, DrawCommand, CanvasId};
use super::binary::{BinaryTransfer, ImageFormat, RenderResult};
use std::sync::Arc;
use uuid::Uuid;

#[command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn init_canvas(
    width: u32,
    height: u32,
    engine: State<'_, Arc<DrawingEngine>>,
) -> Result<String, String> {
    let canvas_id = engine.create_canvas(width, height).await?;
    Ok(canvas_id.0.to_string())
}

#[command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn draw_command(
    command: DrawCommand,
    engine: State<'_, Arc<DrawingEngine>>,
) -> Result<(), String> {
    engine.process_command(command).await
}

#[command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn get_render_result(
    canvas_id: String,
    engine: State<'_, Arc<DrawingEngine>>,
) -> Result<RenderResult, String> {
    let id = Uuid::parse_str(&canvas_id)
        .map_err(|e| format!("Invalid canvas ID: {}", e))?;
    let canvas_id = CanvasId(id);
    
    let (image_data, width, height) = engine.get_canvas_data(&canvas_id).await?;
    
    let transfer = BinaryTransfer::new(image_data, width, height, ImageFormat::Rgba8);
    
    Ok(RenderResult {
        canvas_id: canvas_id.0.to_string(),
        image_data: transfer,
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    })
}

#[command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn resize_canvas(
    canvas_id: String,
    width: u32,
    height: u32,
    engine: State<'_, Arc<DrawingEngine>>,
) -> Result<(), String> {
    let id = Uuid::parse_str(&canvas_id)
        .map_err(|e| format!("Invalid canvas ID: {}", e))?;
    let canvas_id = CanvasId(id);
    
    engine.resize_canvas(&canvas_id, width, height).await
}

#[command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn get_compressed_render_result(
    canvas_id: String,
    engine: State<'_, Arc<DrawingEngine>>,
) -> Result<RenderResult, String> {
    let id = Uuid::parse_str(&canvas_id)
        .map_err(|e| format!("Invalid canvas ID: {}", e))?;
    let canvas_id = CanvasId(id);
    
    let (image_data, width, height) = engine.get_canvas_data(&canvas_id).await?;
    
    let transfer = BinaryTransfer::new_compressed(image_data, width, height, ImageFormat::Rgba8)
        .map_err(|e| format!("Compression failed: {}", e))?;
    
    Ok(RenderResult {
        canvas_id: canvas_id.0.to_string(),
        image_data: transfer,
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    })
}