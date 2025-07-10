use crate::types::{BrushStamp, StampCache, MAX_STAMP_CACHE_SIZE};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// ブラシスタンプを作成
pub fn create_brush_stamp(size: u32) -> BrushStamp {
    let mut alpha_map = Vec::with_capacity((size * size) as usize);
    let half_size = size as f32 / 2.0;
    
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - half_size + 0.5;
            let dy = y as f32 - half_size + 0.5;
            let distance = (dx * dx + dy * dy).sqrt();
            
            // 滑らかなグラデーション（コサインカーブ）
            let alpha = if distance <= half_size {
                let normalized = distance / half_size;
                ((1.0 - normalized) * std::f32::consts::PI).cos() * 0.5 + 0.5
            } else {
                0.0
            };
            
            alpha_map.push(alpha);
        }
    }
    
    BrushStamp { size, alpha_map }
}

/// スタンプを取得または作成
pub fn get_or_create_stamp(cache: &StampCache, size: u32) -> BrushStamp {
    let mut cache_guard = cache.lock().unwrap();
    
    if let Some(stamp) = cache_guard.get(&size) {
        return stamp.clone();
    }
    
    // 新しいスタンプを作成
    let stamp = create_brush_stamp(size);
    
    // キャッシュサイズをチェック
    if cache_guard.len() >= MAX_STAMP_CACHE_SIZE {
        // 最も古いエントリを削除（簡易的な実装）
        if let Some(&oldest_key) = cache_guard.keys().next() {
            cache_guard.remove(&oldest_key);
        }
    }
    
    cache_guard.insert(size, stamp.clone());
    stamp
}