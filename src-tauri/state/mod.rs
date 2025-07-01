use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// アプリケーション全体の状態
#[derive(Debug, Clone)]
pub struct AppState {
    /// プロジェクト情報
    pub project_name: String,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub frame_rate: f32,
    
    /// レイヤー情報
    pub layers: Vec<Layer>,
    pub active_layer_id: Option<String>,
    
    /// フレーム情報（アニメーション用）
    pub frames: Vec<Frame>,
    pub current_frame_index: usize,
    
    /// 操作履歴（Undo/Redo用）
    pub history: OperationHistory,
    
    /// 選択範囲
    pub selection: Option<Selection>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            project_name: "Untitled".to_string(),
            canvas_width: 1920,
            canvas_height: 1080,
            frame_rate: 24.0,
            layers: vec![],
            active_layer_id: None,
            frames: vec![Frame::new(0)],
            current_frame_index: 0,
            history: OperationHistory::new(),
            selection: None,
        }
    }
}

/// レイヤー情報
#[derive(Debug, Clone)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub blend_mode: BlendMode,
}

/// ブレンドモード
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
}

impl ToString for BlendMode {
    fn to_string(&self) -> String {
        match self {
            BlendMode::Normal => "normal".to_string(),
            BlendMode::Multiply => "multiply".to_string(),
            BlendMode::Screen => "screen".to_string(),
            BlendMode::Overlay => "overlay".to_string(),
        }
    }
}

/// フレーム情報
#[derive(Debug, Clone)]
pub struct Frame {
    pub index: usize,
    pub layer_data: HashMap<String, LayerData>,
}

impl Frame {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            layer_data: HashMap::new(),
        }
    }
}

/// レイヤーのピクセルデータ
#[derive(Debug, Clone)]
pub struct LayerData {
    /// ラスターデータ（RGBA）
    pub raster_data: Option<Vec<u8>>,
    /// ベクターオブジェクト
    pub vector_objects: Vec<VectorObject>,
}

/// ベクターオブジェクト
#[derive(Debug, Clone)]
pub enum VectorObject {
    Path {
        points: Vec<PathPoint>,
        color: String,
        width: f32,
    },
    Shape {
        shape_type: ShapeType,
        bounds: Bounds,
        color: String,
        fill: bool,
    },
}

/// パスの点
#[derive(Debug, Clone)]
pub struct PathPoint {
    pub x: f64,
    pub y: f64,
    pub pressure: f32,
}

/// 図形タイプ
#[derive(Debug, Clone)]
pub enum ShapeType {
    Rectangle,
    Ellipse,
    Line,
}

/// 境界ボックス
#[derive(Debug, Clone)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// 選択範囲
#[derive(Debug, Clone)]
pub struct Selection {
    pub selection_type: SelectionType,
    pub points: Vec<(f64, f64)>,
    pub bounds: Bounds,
}

/// 選択範囲のタイプ
#[derive(Debug, Clone)]
pub enum SelectionType {
    Rectangle,
    Lasso,
    Magic,
}

/// 操作履歴
#[derive(Debug, Clone)]
pub struct OperationHistory {
    pub undo_stack: Vec<Operation>,
    pub redo_stack: Vec<Operation>,
    pub max_history: usize,
}

impl OperationHistory {
    pub fn new() -> Self {
        Self {
            undo_stack: vec![],
            redo_stack: vec![],
            max_history: 100,
        }
    }
    
    pub fn add_operation(&mut self, operation: Operation) {
        self.undo_stack.push(operation);
        self.redo_stack.clear();
        
        // 履歴の最大数を超えた場合は古いものから削除
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }
    
    pub fn undo(&mut self) -> Option<Operation> {
        if let Some(operation) = self.undo_stack.pop() {
            self.redo_stack.push(operation.clone());
            Some(operation)
        } else {
            None
        }
    }
    
    pub fn redo(&mut self) -> Option<Operation> {
        if let Some(operation) = self.redo_stack.pop() {
            self.undo_stack.push(operation.clone());
            Some(operation)
        } else {
            None
        }
    }
}

/// 操作の記録
#[derive(Debug, Clone)]
pub enum Operation {
    DrawStroke {
        layer_id: String,
        stroke: VectorObject,
    },
    CreateLayer {
        layer: Layer,
        index: usize,
    },
    DeleteLayer {
        layer: Layer,
        index: usize,
    },
    ModifyLayerProperty {
        layer_id: String,
        old_state: LayerPropertyState,
        new_state: LayerPropertyState,
    },
}

/// レイヤープロパティの状態
#[derive(Debug, Clone)]
pub struct LayerPropertyState {
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub visible: bool,
}