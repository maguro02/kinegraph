use uuid::Uuid;
use super::commands::BlendMode;

pub struct Layer {
    pub id: Uuid,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // RGBA8 format
    pub opacity: f32,
    pub visible: bool,
    pub name: String,
    pub blend_mode: BlendMode,
}

impl Layer {
    pub fn new(width: u32, height: u32) -> Self {
        let data_size = (width * height * 4) as usize;
        Self {
            id: Uuid::new_v4(),
            width,
            height,
            data: vec![0; data_size], // 透明な黒で初期化
            opacity: 1.0,
            visible: true,
            name: "Layer".to_string(),
            blend_mode: BlendMode::Normal,
        }
    }
    
    pub fn clear(&mut self) {
        self.data.fill(0);
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        // 新しいサイズのバッファを作成
        let new_size = (width * height * 4) as usize;
        let mut new_data = vec![0; new_size];
        
        // 既存のデータをコピー（共通部分のみ）
        let min_width = self.width.min(width);
        let min_height = self.height.min(height);
        
        for y in 0..min_height {
            let src_offset = (y * self.width * 4) as usize;
            let dst_offset = (y * width * 4) as usize;
            let copy_size = (min_width * 4) as usize;
            
            new_data[dst_offset..dst_offset + copy_size]
                .copy_from_slice(&self.data[src_offset..src_offset + copy_size]);
        }
        
        self.width = width;
        self.height = height;
        self.data = new_data;
    }
    
    pub fn set_pixel(&mut self, x: u32, y: u32, color: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        
        let offset = ((y * self.width + x) * 4) as usize;
        self.data[offset..offset + 4].copy_from_slice(&color);
    }
    
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        
        let offset = ((y * self.width + x) * 4) as usize;
        let mut pixel = [0u8; 4];
        pixel.copy_from_slice(&self.data[offset..offset + 4]);
        Some(pixel)
    }
}