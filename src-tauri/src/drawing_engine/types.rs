use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// ブラシスタンプのアルファマップ
#[derive(Clone)]
pub struct BrushStamp {
    pub size: u32,
    pub alpha_map: Vec<f32>,
}

impl BrushStamp {
    /// 新しいスタンプを作成
    pub fn new(size: u32) -> Self {
        let mut alpha_map = Vec::with_capacity((size * size) as usize);
        let radius = size as f32 / 2.0;
        
        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 - radius + 0.5;
                let dy = y as f32 - radius + 0.5;
                let distance = (dx * dx + dy * dy).sqrt();
                
                if distance <= radius {
                    // 中心に向かって濃くなるグラデーション
                    // コサインカーブで滑らかな減衰
                    let normalized_distance = distance / radius;
                    let alpha = if normalized_distance <= 0.0 {
                        1.0
                    } else if normalized_distance >= 1.0 {
                        0.0
                    } else {
                        // コサインカーブで滑らかなフォールオフ
                        (1.0 + (normalized_distance * std::f32::consts::PI).cos()) * 0.5
                    };
                    alpha_map.push(alpha);
                } else {
                    alpha_map.push(0.0);
                }
            }
        }
        
        Self { size, alpha_map }
    }
    
    /// 指定座標のアルファ値を取得
    pub fn get_alpha(&self, x: u32, y: u32) -> f32 {
        if x >= self.size || y >= self.size {
            return 0.0;
        }
        self.alpha_map[(y * self.size + x) as usize]
    }
}

/// スタンプキャッシュの型定義
pub type StampCache = Arc<Mutex<HashMap<u32, BrushStamp>>>;

/// スタンプキャッシュの最大サイズ
pub const MAX_STAMP_CACHE_SIZE: usize = 50;