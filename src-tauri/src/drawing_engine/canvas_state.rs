use serde::{Deserialize, Serialize};
use uuid::Uuid;
use super::layer::Layer;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct CanvasId(pub Uuid);

impl CanvasId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CanvasId {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CanvasState {
    pub id: CanvasId,
    pub width: u32,
    pub height: u32,
    pub layers: Vec<Layer>,
    pub active_layer: usize,
    pub texture: Option<wgpu::Texture>,
    pub dirty: bool,
}

impl CanvasState {
    pub fn new(id: CanvasId, width: u32, height: u32) -> Self {
        let mut layers = Vec::new();
        layers.push(Layer::new(width, height));
        
        Self {
            id,
            width,
            height,
            layers,
            active_layer: 0,
            texture: None,
            dirty: true,
        }
    }
    
    pub fn get_active_layer(&mut self) -> Option<&mut Layer> {
        self.layers.get_mut(self.active_layer)
    }
    
    pub fn add_layer(&mut self) -> usize {
        let layer = Layer::new(self.width, self.height);
        self.layers.push(layer);
        self.dirty = true;
        self.layers.len() - 1
    }
    
    pub fn delete_layer(&mut self, index: usize) -> Result<(), String> {
        if self.layers.len() <= 1 {
            return Err("Cannot delete the last layer".to_string());
        }
        
        if index >= self.layers.len() {
            return Err("Layer index out of bounds".to_string());
        }
        
        self.layers.remove(index);
        
        // アクティブレイヤーの調整
        if self.active_layer >= self.layers.len() {
            self.active_layer = self.layers.len() - 1;
        }
        
        self.dirty = true;
        Ok(())
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        
        // すべてのレイヤーをリサイズ
        for layer in &mut self.layers {
            layer.resize(width, height);
        }
        
        self.texture = None; // テクスチャの再作成が必要
        self.dirty = true;
    }
}