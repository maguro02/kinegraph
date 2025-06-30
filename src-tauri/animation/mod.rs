use serde::{Deserialize, Serialize};
#[cfg(feature = "specta")]
use specta::Type;
use chrono;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct Frame {
    pub id: String,
    pub layers: Vec<Layer>,
    pub duration: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct Project {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f32,
    pub frames: Vec<Frame>,
}

impl Project {
    pub fn new(name: String, width: u32, height: u32, frame_rate: f32) -> Self {
        // 初期フレームを作成
        let initial_frame = Frame {
            id: format!("frame_{}", chrono::Utc::now().timestamp_millis()),
            layers: Vec::new(),
            duration: 1.0 / frame_rate, // 1フレーム分の時間
        };
        
        Self {
            name,
            width,
            height,
            frame_rate,
            frames: vec![initial_frame], // 初期フレームを含める
        }
    }
}