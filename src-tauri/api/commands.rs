use serde::{Deserialize, Serialize};

/// ユーザー操作を表す入力コマンド
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum UserInput {
    /// ストロークを描画
    DrawStroke {
        points: Vec<Point>,
        color: String,
        width: f32,
        layer_id: String,
    },
    /// 現在のツールを変更
    ChangeTool { tool_id: String },
    /// レイヤーを作成
    CreateLayer { name: String },
    /// レイヤーを削除
    DeleteLayer { layer_id: String },
    /// レイヤーの順序を変更
    ReorderLayer { layer_id: String, new_index: usize },
    /// レイヤーの不透明度を変更
    ChangeLayerOpacity { layer_id: String, opacity: f32 },
    /// レイヤーのブレンドモードを変更
    ChangeLayerBlendMode { layer_id: String, blend_mode: String },
    /// 塗りつぶし
    Fill { 
        point: Point, 
        color: String,
        layer_id: String,
    },
    /// 選択範囲を作成
    CreateSelection { 
        selection_type: SelectionType,
        points: Vec<Point>,
    },
    /// 選択範囲を変形
    TransformSelection {
        transform: Transform,
    },
    /// Undo操作
    Undo,
    /// Redo操作
    Redo,
}

/// 描画コマンド（フロントエンドへの描画指示）
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum DrawCommand {
    /// キャンバス全体をクリア
    ClearCanvas,
    /// パスを描画
    DrawPath { 
        points: Vec<Point>, 
        color: String, 
        width: f32,
        layer_id: String,
    },
    /// 矩形領域を更新（部分的なラスターデータ更新用）
    UpdateRasterArea { 
        rect: Rect, 
        pixel_data: Vec<u8>,
        layer_id: String,
    },
    /// レイヤーを追加
    AddLayer {
        layer_id: String,
        index: usize,
    },
    /// レイヤーを削除
    RemoveLayer {
        layer_id: String,
    },
    /// レイヤーの順序を変更
    ReorderLayers {
        layer_ids: Vec<String>,
    },
    /// レイヤーのプロパティを更新
    UpdateLayerProperties {
        layer_id: String,
        opacity: f32,
        blend_mode: String,
        visible: bool,
    },
    /// 選択範囲を表示
    ShowSelection {
        selection_type: SelectionType,
        points: Vec<Point>,
    },
    /// 選択範囲をクリア
    ClearSelection,
    /// 変形マトリックスを適用
    ApplyTransform {
        layer_id: String,
        transform: Transform,
    },
    /// 複数のコマンドをバッチ実行
    Batch {
        commands: Vec<DrawCommand>,
    },
}

/// 2D座標
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// 矩形領域
#[derive(Debug, Serialize, Clone)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// 選択範囲のタイプ
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum SelectionType {
    Rectangle,
    Lasso,
    Magic,
}

/// 変形情報
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transform {
    pub translate_x: f64,
    pub translate_y: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub rotation: f64, // ラジアン
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translate_x: 0.0,
            translate_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation: 0.0,
        }
    }
}