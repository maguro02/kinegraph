use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum DrawCommand {
    BeginStroke { x: f32, y: f32, pressure: f32 },
    ContinueStroke { x: f32, y: f32, pressure: f32 },
    EndStroke,
    Clear,
    SetBrush(BrushSettings),
    SetActiveLayer(usize),
    CreateLayer,
    DeleteLayer(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct BrushSettings {
    pub size: f32,
    pub opacity: f32,
    pub color: [f32; 4], // RGBA
    pub brush_type: BrushType,
    pub blend_mode: BlendMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum BrushType {
    Pen,
    Brush,
    Eraser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            size: 5.0,
            opacity: 1.0,
            color: [0.0, 0.0, 0.0, 1.0],
            brush_type: BrushType::Pen,
            blend_mode: BlendMode::Normal,
        }
    }
}