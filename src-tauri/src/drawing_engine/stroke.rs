use crate::drawing_engine::commands::BrushSettings;
use crate::drawing_engine::types::{BrushStamp, StampCache, MAX_STAMP_CACHE_SIZE};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

/// スタンプキャッシュ
static STAMP_CACHE: Lazy<StampCache> = Lazy::new(|| {
    Arc::new(Mutex::new(HashMap::new()))
});

/// ブラシスタンプを作成
pub fn create_brush_stamp(size: u32) -> BrushStamp {
    BrushStamp::new(size)
}

/// スタンプを取得（キャッシュから、なければ作成）
pub fn get_or_create_stamp(size: u32) -> BrushStamp {
    let mut cache = STAMP_CACHE.lock().unwrap();
    
    // キャッシュにあれば返す
    if let Some(stamp) = cache.get(&size) {
        return stamp.clone();
    }
    
    // なければ作成してキャッシュ
    let stamp = create_brush_stamp(size);
    cache.insert(size, stamp.clone());
    
    // キャッシュサイズの制限（メモリ効率のため）
    if cache.len() > MAX_STAMP_CACHE_SIZE {
        // 古いエントリを削除（簡易的にランダムに1つ削除）
        if let Some(key) = cache.keys().next().cloned() {
            cache.remove(&key);
        }
    }
    
    stamp
}

#[derive(Debug, Clone)]
pub struct StrokeState {
    pub is_drawing: bool,
    pub last_point: Option<(f32, f32)>,
    pub current_path: Vec<(f32, f32, f32)>, // x, y, pressure
    pub last_stamp_position: Option<(f32, f32)>, // 最後にスタンプを配置した位置
    pub last_rendered_index: usize, // 最後にレンダリングされたパスのインデックス
    pub dirty_region: Option<DirtyRegion>, // 更新が必要な領域
}

/// ダーティリージョン（更新が必要な領域）
#[derive(Debug, Clone)]
pub struct DirtyRegion {
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
}

impl DirtyRegion {
    pub fn new() -> Self {
        Self {
            min_x: u32::MAX,
            min_y: u32::MAX,
            max_x: 0,
            max_y: 0,
        }
    }
    
    pub fn expand(&mut self, x: f32, y: f32, radius: f32) {
        let left = (x - radius).max(0.0) as u32;
        let top = (y - radius).max(0.0) as u32;
        let right = (x + radius).ceil() as u32;
        let bottom = (y + radius).ceil() as u32;
        
        self.min_x = self.min_x.min(left);
        self.min_y = self.min_y.min(top);
        self.max_x = self.max_x.max(right);
        self.max_y = self.max_y.max(bottom);
    }
    
    pub fn is_valid(&self) -> bool {
        self.min_x <= self.max_x && self.min_y <= self.max_y
    }
    
    pub fn reset(&mut self) {
        self.min_x = u32::MAX;
        self.min_y = u32::MAX;
        self.max_x = 0;
        self.max_y = 0;
    }
}

impl Default for StrokeState {
    fn default() -> Self {
        Self {
            is_drawing: false,
            last_point: None,
            current_path: Vec::new(),
            last_stamp_position: None,
            last_rendered_index: 0,
            dirty_region: None,
        }
    }
}

impl StrokeState {
    pub fn begin(&mut self, x: f32, y: f32, pressure: f32) {
        self.is_drawing = true;
        self.last_point = Some((x, y));
        self.current_path.clear();
        self.current_path.push((x, y, pressure));
        self.last_stamp_position = Some((x, y));
        self.last_rendered_index = 0;
        self.dirty_region = Some(DirtyRegion::new());
    }
    
    pub fn add_point(&mut self, x: f32, y: f32, pressure: f32) {
        if self.is_drawing {
            self.current_path.push((x, y, pressure));
            self.last_point = Some((x, y));
        }
    }
    
    pub fn end(&mut self) {
        self.is_drawing = false;
        self.last_point = None;
        self.last_stamp_position = None;
        // パスデータは保持し、last_rendered_indexもリセットしない
        // これにより、完成したストロークの情報が保持される
    }
    
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    
    /// 未レンダリングのポイントがあるかチェック
    pub fn has_unrendered_points(&self) -> bool {
        self.last_rendered_index < self.current_path.len()
    }
    
    /// 次にレンダリングすべきポイントの範囲を取得
    pub fn get_unrendered_range(&self) -> Option<(usize, usize)> {
        if self.has_unrendered_points() {
            Some((self.last_rendered_index, self.current_path.len()))
        } else {
            None
        }
    }
    
    /// レンダリング完了を記録
    pub fn mark_rendered(&mut self, up_to_index: usize) {
        self.last_rendered_index = self.last_rendered_index.max(up_to_index);
    }
}

/// ブレゼンハムのアルゴリズムを使用した線の補間
pub fn interpolate_line(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
    let mut points = Vec::new();
    
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    let mut x = x0;
    let mut y = y0;
    
    loop {
        points.push((x, y));
        
        if x == x1 && y == y1 {
            break;
        }
        
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
    
    points
}

/// 圧力を考慮したブラシストロークの描画（レガシー実装）
pub fn draw_brush_stroke(
    layer_data: &mut [u8],
    width: u32,
    height: u32,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    pressure0: f32,
    pressure1: f32,
    brush: &BrushSettings,
) {
    // スタンプベース描画を使用
    draw_line_with_stamps(layer_data, width, height, x0, y0, x1, y1, pressure0, pressure1, brush);
}

/// スタンプベースのライン描画
pub fn draw_line_with_stamps(
    layer_data: &mut [u8],
    width: u32,
    height: u32,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    pressure0: f32,
    pressure1: f32,
    brush: &BrushSettings,
) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let distance = (dx * dx + dy * dy).sqrt();
    
    if distance < 0.01 {
        // 点の場合は単一スタンプ
        draw_stamp(layer_data, width, height, x1, y1, pressure1, brush);
        return;
    }
    
    // 基本のスタンプ間隔（ブラシサイズの1/4）
    let base_spacing = brush.size * 0.25;
    
    // 速度に基づく動的間隔調整
    // 速い動きではスタンプ間隔を広げて重複を減らす
    let velocity = distance; // フレーム間の距離を速度の指標として使用
    let velocity_factor = (velocity / 50.0).clamp(0.5, 2.0); // 0.5〜2.0の範囲で調整
    let spacing = base_spacing * velocity_factor;
    
    // 必要なスタンプ数を計算
    let num_stamps = (distance / spacing).ceil() as usize;
    let num_stamps = num_stamps.max(2); // 最低2つのスタンプ
    
    // 各スタンプを配置
    for i in 0..num_stamps {
        let t = i as f32 / (num_stamps - 1).max(1) as f32;
        let x = x0 + dx * t;
        let y = y0 + dy * t;
        let pressure = pressure0 * (1.0 - t) + pressure1 * t;
        
        draw_stamp(layer_data, width, height, x, y, pressure, brush);
    }
}

/// インクリメンタルなライン描画（差分のみレンダリング）
pub fn draw_line_incremental(
    layer_data: &mut [u8],
    width: u32,
    height: u32,
    stroke_state: &mut StrokeState,
    brush: &BrushSettings,
) -> Option<DirtyRegion> {
    // 未レンダリングのポイントを取得
    let (start_idx, end_idx) = stroke_state.get_unrendered_range()?;
    
    if start_idx >= end_idx || start_idx >= stroke_state.current_path.len() {
        return None;
    }
    
    let mut local_dirty = DirtyRegion::new();
    
    // 開始ポイントから差分描画
    for i in start_idx..end_idx {
        if i == 0 {
            // 最初のポイントは単体で描画
            let (x, y, pressure) = stroke_state.current_path[i];
            draw_stamp(layer_data, width, height, x, y, pressure, brush);
            local_dirty.expand(x, y, brush.size * pressure);
        } else {
            // 前のポイントからラインを描画
            let (x0, y0, pressure0) = stroke_state.current_path[i - 1];
            let (x1, y1, pressure1) = stroke_state.current_path[i];
            
            // スタンプベースでインクリメンタルに描画
            draw_line_segment_incremental(
                layer_data,
                width,
                height,
                x0, y0, x1, y1,
                pressure0, pressure1,
                brush,
                &mut local_dirty,
            );
        }
    }
    
    // レンダリング完了を記録
    stroke_state.mark_rendered(end_idx);
    
    // グローバルなダーティリージョンを更新
    if let Some(ref mut global_dirty) = stroke_state.dirty_region {
        global_dirty.min_x = global_dirty.min_x.min(local_dirty.min_x);
        global_dirty.min_y = global_dirty.min_y.min(local_dirty.min_y);
        global_dirty.max_x = global_dirty.max_x.max(local_dirty.max_x);
        global_dirty.max_y = global_dirty.max_y.max(local_dirty.max_y);
    }
    
    Some(local_dirty)
}

/// ラインセグメントをインクリメンタルに描画
fn draw_line_segment_incremental(
    layer_data: &mut [u8],
    width: u32,
    height: u32,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    pressure0: f32,
    pressure1: f32,
    brush: &BrushSettings,
    dirty_region: &mut DirtyRegion,
) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let distance = (dx * dx + dy * dy).sqrt();
    
    if distance < 0.01 {
        draw_stamp(layer_data, width, height, x1, y1, pressure1, brush);
        dirty_region.expand(x1, y1, brush.size * pressure1);
        return;
    }
    
    // 基本のスタンプ間隔
    let base_spacing = brush.size * 0.25;
    let velocity = distance;
    let velocity_factor = (velocity / 50.0).clamp(0.5, 2.0);
    let spacing = base_spacing * velocity_factor;
    
    let num_stamps = (distance / spacing).ceil() as usize;
    let num_stamps = num_stamps.max(2);
    
    // 各スタンプを配置
    for i in 0..num_stamps {
        let t = i as f32 / (num_stamps - 1).max(1) as f32;
        let x = x0 + dx * t;
        let y = y0 + dy * t;
        let pressure = pressure0 * (1.0 - t) + pressure1 * t;
        
        draw_stamp(layer_data, width, height, x, y, pressure, brush);
        dirty_region.expand(x, y, brush.size * pressure);
    }
}

/// 単一のスタンプを描画
pub fn draw_stamp(
    layer_data: &mut [u8],
    width: u32,
    height: u32,
    x: f32,
    y: f32,
    pressure: f32,
    brush: &BrushSettings,
) {
    let size = (brush.size * pressure).max(1.0) as u32;
    let stamp = get_or_create_stamp(size);
    
    let half_size = size as i32 / 2;
    let center_x = x as i32;
    let center_y = y as i32;
    
    // スタンプの範囲内のピクセルを塗る
    for stamp_y in 0..size {
        for stamp_x in 0..size {
            let px = center_x - half_size + stamp_x as i32;
            let py = center_y - half_size + stamp_y as i32;
            
            if px >= 0 && py >= 0 && px < width as i32 && py < height as i32 {
                let stamp_alpha = stamp.get_alpha(stamp_x, stamp_y);
                
                if stamp_alpha > 0.0 {
                    let alpha = brush.opacity * pressure * stamp_alpha;
                    let offset = ((py as u32 * width + px as u32) * 4) as usize;
                    
                    match brush.brush_type {
                        crate::drawing_engine::commands::BrushType::Pen |
                        crate::drawing_engine::commands::BrushType::Brush => {
                            // 通常の描画
                            blend_pixel(
                                &mut layer_data[offset..offset + 4],
                                brush.color,
                                alpha,
                                &brush.blend_mode,
                            );
                        }
                        crate::drawing_engine::commands::BrushType::Eraser => {
                            // 消しゴム
                            let current_alpha = layer_data[offset + 3] as f32 / 255.0;
                            layer_data[offset + 3] = ((current_alpha * (1.0 - alpha)) * 255.0) as u8;
                        }
                    }
                }
            }
        }
    }
}

/// 単一のブラシポイントを描画（スタンプベース実装を使用）
pub fn draw_brush_point(
    layer_data: &mut [u8],
    width: u32,
    height: u32,
    x: f32,
    y: f32,
    pressure: f32,
    brush: &BrushSettings,
) {
    // スタンプベース描画を使用
    draw_stamp(layer_data, width, height, x, y, pressure, brush);
}

/// ピクセルのブレンド処理
fn blend_pixel(
    dst: &mut [u8],
    src_color: [f32; 4],
    alpha: f32,
    blend_mode: &crate::drawing_engine::commands::BlendMode,
) {
    let src_r = (src_color[0] * 255.0) as u8;
    let src_g = (src_color[1] * 255.0) as u8;
    let src_b = (src_color[2] * 255.0) as u8;
    let src_a = (alpha * 255.0) as u8;
    
    let dst_r = dst[0];
    let dst_g = dst[1];
    let dst_b = dst[2];
    let dst_a = dst[3];
    
    let (new_r, new_g, new_b) = match blend_mode {
        crate::drawing_engine::commands::BlendMode::Normal => {
            // 通常のアルファブレンド
            let inv_alpha = 1.0 - alpha;
            (
                (src_r as f32 * alpha + dst_r as f32 * inv_alpha) as u8,
                (src_g as f32 * alpha + dst_g as f32 * inv_alpha) as u8,
                (src_b as f32 * alpha + dst_b as f32 * inv_alpha) as u8,
            )
        }
        crate::drawing_engine::commands::BlendMode::Multiply => {
            // 乗算
            (
                ((src_r as f32 * dst_r as f32 / 255.0) * alpha + dst_r as f32 * (1.0 - alpha)) as u8,
                ((src_g as f32 * dst_g as f32 / 255.0) * alpha + dst_g as f32 * (1.0 - alpha)) as u8,
                ((src_b as f32 * dst_b as f32 / 255.0) * alpha + dst_b as f32 * (1.0 - alpha)) as u8,
            )
        }
        crate::drawing_engine::commands::BlendMode::Screen => {
            // スクリーン
            (
                (255.0 - ((255.0 - src_r as f32) * (255.0 - dst_r as f32) / 255.0) * alpha 
                    - dst_r as f32 * (1.0 - alpha)) as u8,
                (255.0 - ((255.0 - src_g as f32) * (255.0 - dst_g as f32) / 255.0) * alpha 
                    - dst_g as f32 * (1.0 - alpha)) as u8,
                (255.0 - ((255.0 - src_b as f32) * (255.0 - dst_b as f32) / 255.0) * alpha 
                    - dst_b as f32 * (1.0 - alpha)) as u8,
            )
        }
        crate::drawing_engine::commands::BlendMode::Overlay => {
            // オーバーレイ
            fn overlay_channel(src: f32, dst: f32, alpha: f32) -> u8 {
                let result = if dst < 128.0 {
                    2.0 * src * dst / 255.0
                } else {
                    255.0 - 2.0 * (255.0 - src) * (255.0 - dst) / 255.0
                };
                (result * alpha + dst * (1.0 - alpha)) as u8
            }
            
            (
                overlay_channel(src_r as f32, dst_r as f32, alpha),
                overlay_channel(src_g as f32, dst_g as f32, alpha),
                overlay_channel(src_b as f32, dst_b as f32, alpha),
            )
        }
    };
    
    dst[0] = new_r;
    dst[1] = new_g;
    dst[2] = new_b;
    dst[3] = ((src_a as f32 + dst_a as f32 * (1.0 - alpha)) as u8).min(255);
}