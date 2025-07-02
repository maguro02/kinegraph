use serde::{Serialize, Deserialize};

/// 2Dポイント
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y, pressure: 1.0 }
    }
    
    pub fn with_pressure(x: f32, y: f32, pressure: f32) -> Self {
        Self { x, y, pressure }
    }
    
    /// 2点間の距離を計算
    pub fn distance_to(&self, other: &Point) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// RGBAカラー
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
    
    /// 黒色
    pub fn black() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }
    
    /// 白色
    pub fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }
    
    /// 透明
    pub fn transparent() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }
    
    /// 16進数文字列からColorを作成
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 && hex.len() != 8 {
            return None;
        }
        
        let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16).ok()? as f32 / 255.0
        } else {
            1.0
        };
        
        Some(Self::new(r, g, b, a))
    }
    
    /// [r, g, b, a]配列として取得
    pub fn as_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// ブラシタイプ
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrushType {
    /// 基本のペン
    Pen,
    /// 鉄筆
    Pencil,
    /// マーカー
    Marker,
    /// 消しゴム
    Eraser,
    /// カスタムブラシ
    Custom(String),
}

impl Default for BrushType {
    fn default() -> Self {
        Self::Pen
    }
}

/// 完成したストローク
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stroke {
    /// ストロークID
    pub id: u32,
    /// ポイントの配列
    pub points: Vec<Point>,
    /// ストロークの色
    pub color: Color,
    /// ブラシタイプ
    pub brush_type: BrushType,
    /// ブラシサイズ
    pub size: f32,
}

impl Stroke {
    /// ストロークのバウンディングボックスを計算
    pub fn bounding_box(&self) -> Option<(Point, Point)> {
        if self.points.is_empty() {
            return None;
        }
        
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        
        for point in &self.points {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }
        
        // ブラシサイズを考慮
        let half_size = self.size / 2.0;
        min_x -= half_size;
        min_y -= half_size;
        max_x += half_size;
        max_y += half_size;
        
        Some((Point::new(min_x, min_y), Point::new(max_x, max_y)))
    }
}

/// アクティブな（描画中の）ストローク
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveStroke {
    /// ストロークID
    pub id: u32,
    /// ポイントの配列
    pub points: Vec<Point>,
    /// ストロークの色
    pub color: Color,
    /// ブラシタイプ
    pub brush_type: BrushType,
    /// ブラシサイズ
    pub size: f32,
}

impl ActiveStroke {
    /// ActiveStrokeをStrokeに変換
    pub fn into_stroke(self) -> Stroke {
        Stroke {
            id: self.id,
            points: self.points,
            color: self.color,
            brush_type: self.brush_type,
            size: self.size,
        }
    }
}

/// 再描画が必要な領域
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirtyRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl DirtyRegion {
    /// 2つのDirtyRegionをマージ
    pub fn merge(&self, other: &DirtyRegion) -> DirtyRegion {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = (self.x + self.width).max(other.x + other.width);
        let y2 = (self.y + self.height).max(other.y + other.height);
        
        DirtyRegion {
            x: x1,
            y: y1,
            width: x2 - x1,
            height: y2 - y1,
        }
    }
    
    /// 領域が重なっているか判定
    pub fn intersects(&self, other: &DirtyRegion) -> bool {
        let x1 = self.x;
        let y1 = self.y;
        let x2 = self.x + self.width;
        let y2 = self.y + self.height;
        
        let ox1 = other.x;
        let oy1 = other.y;
        let ox2 = other.x + other.width;
        let oy2 = other.y + other.height;
        
        x1 < ox2 && x2 > ox1 && y1 < oy2 && y2 > oy1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_point_distance() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(3.0, 4.0);
        assert_eq!(p1.distance_to(&p2), 5.0);
    }
    
    #[test]
    fn test_color_from_hex() {
        let color = Color::from_hex("#FF0000").unwrap();
        assert_eq!(color.r, 1.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 0.0);
        assert_eq!(color.a, 1.0);
        
        let color_with_alpha = Color::from_hex("#FF000080").unwrap();
        assert_eq!(color_with_alpha.a, 128.0 / 255.0);
    }
    
    #[test]
    fn test_dirty_region_merge() {
        let r1 = DirtyRegion { x: 10, y: 10, width: 20, height: 20 };
        let r2 = DirtyRegion { x: 20, y: 20, width: 30, height: 30 };
        let merged = r1.merge(&r2);
        
        assert_eq!(merged.x, 10);
        assert_eq!(merged.y, 10);
        assert_eq!(merged.width, 40);
        assert_eq!(merged.height, 40);
    }
}
