use crate::drawing_engine::commands::BrushSettings;

#[derive(Debug, Clone)]
pub struct StrokeState {
    pub is_drawing: bool,
    pub last_point: Option<(f32, f32)>,
    pub current_path: Vec<(f32, f32, f32)>, // x, y, pressure
}

impl Default for StrokeState {
    fn default() -> Self {
        Self {
            is_drawing: false,
            last_point: None,
            current_path: Vec::new(),
        }
    }
}

impl StrokeState {
    pub fn begin(&mut self, x: f32, y: f32, pressure: f32) {
        self.is_drawing = true;
        self.last_point = Some((x, y));
        self.current_path.clear();
        self.current_path.push((x, y, pressure));
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
    }
    
    pub fn clear(&mut self) {
        *self = Self::default();
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

/// 圧力を考慮したブラシストロークの描画
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
    let points = interpolate_line(x0 as i32, y0 as i32, x1 as i32, y1 as i32);
    let total_points = points.len() as f32;
    
    for (i, &(x, y)) in points.iter().enumerate() {
        // 線形補間で圧力を計算
        let t = i as f32 / total_points.max(1.0);
        let pressure = pressure0 * (1.0 - t) + pressure1 * t;
        
        draw_brush_point(layer_data, width, height, x as f32, y as f32, pressure, brush);
    }
}

/// 単一のブラシポイントを描画
pub fn draw_brush_point(
    layer_data: &mut [u8],
    width: u32,
    height: u32,
    x: f32,
    y: f32,
    pressure: f32,
    brush: &BrushSettings,
) {
    let size = (brush.size * pressure) as i32;
    let half_size = size / 2;
    
    let center_x = x as i32;
    let center_y = y as i32;
    
    // ブラシの範囲内のピクセルを塗る
    for dy in -half_size..=half_size {
        for dx in -half_size..=half_size {
            let px = center_x + dx;
            let py = center_y + dy;
            
            if px >= 0 && py >= 0 && px < width as i32 && py < height as i32 {
                // 円形ブラシの判定
                let dist_sq = (dx * dx + dy * dy) as f32;
                let radius_sq = (half_size * half_size) as f32;
                
                if dist_sq <= radius_sq {
                    // エッジのソフトネス計算
                    let edge_softness = 1.0 - (dist_sq / radius_sq).sqrt();
                    let alpha = brush.opacity * pressure * edge_softness;
                    
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