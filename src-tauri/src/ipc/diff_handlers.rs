use tauri::{command, State};
use crate::drawing_engine::{DrawingEngine, CanvasId};
use crate::drawing_engine::buffer::BufferManager;
use super::binary::{BinaryTransfer, ImageFormat};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct DiffRenderResult {
    pub canvas_id: String,
    pub dirty_region: Option<DirtyRegionData>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct DirtyRegionData {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub image_data: BinaryTransfer,
}

// 前回のレンダリング結果を保存するための構造
pub struct RenderCache {
    cache: Arc<Mutex<HashMap<CanvasId, Vec<u8>>>>,
}

impl RenderCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub async fn get(&self, canvas_id: &CanvasId) -> Option<Vec<u8>> {
        let cache = self.cache.lock().await;
        cache.get(canvas_id).cloned()
    }
    
    pub async fn set(&self, canvas_id: CanvasId, data: Vec<u8>) {
        let mut cache = self.cache.lock().await;
        cache.insert(canvas_id, data);
    }
    
    pub async fn remove(&self, canvas_id: &CanvasId) {
        let mut cache = self.cache.lock().await;
        cache.remove(canvas_id);
    }
}

#[command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn get_diff_render_result(
    canvas_id: String,
    engine: State<'_, Arc<DrawingEngine>>,
    cache: State<'_, Arc<RenderCache>>,
) -> Result<DiffRenderResult, String> {
    let id = Uuid::parse_str(&canvas_id)
        .map_err(|e| format!("Invalid canvas ID: {}", e))?;
    let canvas_id_obj = CanvasId(id);
    
    // 現在のレンダリング結果を取得
    let (current_data, width, height) = engine.get_canvas_data(&canvas_id_obj).await?;
    
    // 前回のレンダリング結果を取得
    let previous_data = cache.get(&canvas_id_obj).await;
    
    let dirty_region = if let Some(prev) = previous_data {
        // 差分領域を計算
        BufferManager::calculate_dirty_region(&prev, &current_data, width, height)
            .map(|region| {
                let transfer = BinaryTransfer::new(
                    region.data,
                    region.width,
                    region.height,
                    ImageFormat::Rgba8,
                );
                
                DirtyRegionData {
                    x: region.x,
                    y: region.y,
                    width: region.width,
                    height: region.height,
                    image_data: transfer,
                }
            })
    } else {
        // 初回は全体を送信
        let transfer = BinaryTransfer::new(
            current_data.clone(),
            width,
            height,
            ImageFormat::Rgba8,
        );
        
        Some(DirtyRegionData {
            x: 0,
            y: 0,
            width,
            height,
            image_data: transfer,
        })
    };
    
    // 現在のデータをキャッシュに保存
    cache.set(canvas_id_obj, current_data).await;
    
    Ok(DiffRenderResult {
        canvas_id,
        dirty_region,
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    })
}

#[command]
#[cfg_attr(feature = "specta", specta::specta)]
pub async fn clear_render_cache(
    canvas_id: String,
    cache: State<'_, Arc<RenderCache>>,
) -> Result<(), String> {
    let id = Uuid::parse_str(&canvas_id)
        .map_err(|e| format!("Invalid canvas ID: {}", e))?;
    let canvas_id = CanvasId(id);
    
    cache.remove(&canvas_id).await;
    Ok(())
}