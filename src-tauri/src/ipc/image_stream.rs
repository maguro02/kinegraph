use crate::drawing_engine::DrawingEngine;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// 画像ヘッダー情報
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ImageHeader {
    pub width: u32,
    pub height: u32,
    pub format: String, // "rgba8", "rgb8", etc.
    pub compressed: bool,
    pub original_size: usize,
    pub compressed_size: Option<usize>,
}

// ストリーミング用のチャンク情報
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ImageChunk {
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub data: Vec<u8>,
}

/// 画像データをバイナリ形式で取得（ストリーミング対応）
pub async fn get_canvas_data_stream(
    engine: Arc<DrawingEngine>,
    canvas_id: &crate::drawing_engine::CanvasId,
    compress: bool,
    _chunk_size: Option<usize>,
) -> Result<(ImageHeader, Vec<u8>), String> {
    // キャンバスデータを取得
    let (rgba_data, width, height) = engine.get_canvas_data(canvas_id).await?;
    let original_size = rgba_data.len();

    debug!(
        "Exporting canvas data: {}x{}, {} bytes",
        width, height, original_size
    );

    // 画像データの圧縮処理
    let (compressed_data, compressed_size) = if compress {
        match compress_image_data(&rgba_data) {
            Ok(compressed) => {
                let size = compressed.len();
                debug!("Compressed image data: {} -> {} bytes", original_size, size);
                (compressed, Some(size))
            }
            Err(e) => {
                error!("Failed to compress image data: {}", e);
                (rgba_data, None)
            }
        }
    } else {
        (rgba_data, None)
    };

    // ヘッダー情報の作成
    let header = ImageHeader {
        width,
        height,
        format: "rgba8".to_string(),
        compressed: compress && compressed_size.is_some(),
        original_size,
        compressed_size,
    };

    Ok((header, compressed_data))
}

/// 画像データをチャンクに分割して送信
pub async fn get_canvas_data_chunked(
    engine: Arc<DrawingEngine>,
    canvas_id: &crate::drawing_engine::CanvasId,
    compress: bool,
    chunk_size: usize,
) -> Result<(ImageHeader, Vec<ImageChunk>), String> {
    let (header, data) = get_canvas_data_stream(engine, canvas_id, compress, None).await?;
    
    // データをチャンクに分割
    let chunks = split_into_chunks(data, chunk_size);
    
    Ok((header, chunks))
}

/// データをチャンクに分割
fn split_into_chunks(data: Vec<u8>, chunk_size: usize) -> Vec<ImageChunk> {
    let total_chunks = (data.len() + chunk_size - 1) / chunk_size;
    let mut chunks = Vec::with_capacity(total_chunks);
    
    for (index, chunk_data) in data.chunks(chunk_size).enumerate() {
        chunks.push(ImageChunk {
            chunk_index: index,
            total_chunks,
            data: chunk_data.to_vec(),
        });
    }
    
    chunks
}

/// LZ4圧縮を使用した画像データの圧縮
fn compress_image_data(data: &[u8]) -> Result<Vec<u8>, String> {
    use lz4::EncoderBuilder;
    use std::io::Write;
    
    let mut encoder = EncoderBuilder::new()
        .level(4) // 中間的な圧縮レベル（速度と圧縮率のバランス）
        .build(Vec::new())
        .map_err(|e| format!("Failed to create LZ4 encoder: {}", e))?;
    
    encoder
        .write_all(data)
        .map_err(|e| format!("Failed to write data to encoder: {}", e))?;
    
    let (compressed, result) = encoder.finish();
    result.map_err(|e| format!("Failed to finish encoding: {}", e))?;
    
    Ok(compressed)
}

/// LZ4圧縮を使用した画像データの解凍
#[allow(dead_code)]
pub fn decompress_image_data(compressed: &[u8]) -> Result<Vec<u8>, String> {
    use lz4::Decoder;
    use std::io::Read;
    
    let mut decoder = Decoder::new(compressed)
        .map_err(|e| format!("Failed to create LZ4 decoder: {}", e))?;
    
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| format!("Failed to decompress data: {}", e))?;
    
    Ok(decompressed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress() {
        let original_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        
        let compressed = compress_image_data(&original_data).unwrap();
        assert!(compressed.len() > 0);
        
        let decompressed = decompress_image_data(&compressed).unwrap();
        assert_eq!(original_data, decompressed);
    }
    
    #[test]
    fn test_split_into_chunks() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let chunks = split_into_chunks(data.clone(), 3);
        
        assert_eq!(chunks.len(), 4);
        assert_eq!(chunks[0].data, vec![1, 2, 3]);
        assert_eq!(chunks[1].data, vec![4, 5, 6]);
        assert_eq!(chunks[2].data, vec![7, 8, 9]);
        assert_eq!(chunks[3].data, vec![10]);
        
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.chunk_index, i);
            assert_eq!(chunk.total_chunks, 4);
        }
    }
}