use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub id: String,
    pub layers: Vec<Layer>,
    pub duration: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f32,
    pub frames: Vec<Frame>,
}

impl Project {
    pub fn new(name: String, width: u32, height: u32, frame_rate: f32) -> Self {
        Self {
            name,
            width,
            height,
            frame_rate,
            frames: Vec::new(),
        }
    }
}