use serde::{Deserialize, Serialize};
use crate::drawing_engine::buffer::BufferManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct BinaryTransfer {
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
    pub compressed: bool,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum ImageFormat {
    Rgba8,
    Rgb8,
}

impl BinaryTransfer {
    pub fn new(data: Vec<u8>, width: u32, height: u32, format: ImageFormat) -> Self {
        Self {
            format,
            width,
            height,
            compressed: false,
            data,
        }
    }
    
    pub fn new_compressed(data: Vec<u8>, width: u32, height: u32, format: ImageFormat) -> Result<Self, Box<dyn std::error::Error>> {
        let compressed_data = BufferManager::compress(&data)?;
        Ok(Self {
            format,
            width,
            height,
            compressed: true,
            data: compressed_data,
        })
    }
    
    pub fn decompress(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if self.compressed {
            BufferManager::decompress(&self.data)
        } else {
            Ok(self.data.clone())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct RenderResult {
    pub canvas_id: String,
    pub image_data: BinaryTransfer,
    pub timestamp: u64,
}