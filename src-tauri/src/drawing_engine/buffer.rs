use lz4::{Decoder, EncoderBuilder};
use std::io::{Read, Write};

/// 差分更新用のデータ構造
pub struct DirtyRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

pub struct BufferManager;

impl BufferManager {
    /// データを圧縮してバイナリ形式で返す
    pub fn compress(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut encoder = EncoderBuilder::new()
            .level(4) // バランスの良い圧縮レベル
            .build(Vec::new())?;
        
        encoder.write_all(data)?;
        let (compressed, _) = encoder.finish();
        Ok(compressed)
    }
    
    /// 圧縮されたデータを解凍する
    pub fn decompress(compressed: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut decoder = Decoder::new(compressed)?;
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }
    
    /// 差分領域を計算
    pub fn calculate_dirty_region(
        old_data: &[u8],
        new_data: &[u8],
        width: u32,
        height: u32,
    ) -> Option<DirtyRegion> {
        let mut min_x = width;
        let mut min_y = height;
        let mut max_x = 0;
        let mut max_y = 0;
        let mut has_changes = false;
        
        // 変更されたピクセルの範囲を検出
        for y in 0..height {
            for x in 0..width {
                let offset = ((y * width + x) * 4) as usize;
                let old_pixel = &old_data[offset..offset + 4];
                let new_pixel = &new_data[offset..offset + 4];
                
                if old_pixel != new_pixel {
                    has_changes = true;
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }
        
        if !has_changes {
            return None;
        }
        
        // 差分領域のデータを抽出
        let region_width = max_x - min_x + 1;
        let region_height = max_y - min_y + 1;
        let mut region_data = Vec::with_capacity((region_width * region_height * 4) as usize);
        
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let offset = ((y * width + x) * 4) as usize;
                region_data.extend_from_slice(&new_data[offset..offset + 4]);
            }
        }
        
        Some(DirtyRegion {
            x: min_x,
            y: min_y,
            width: region_width,
            height: region_height,
            data: region_data,
        })
    }
}