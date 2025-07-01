// ハイブリッド描画エンジンのコマンド定義

// ============================
// UserInput（フロントエンド→バックエンド）
// ============================

export type Point = {
  x: number;
  y: number;
};

export type Transform = {
  translate_x: number;
  translate_y: number;
  scale_x: number;
  scale_y: number;
  rotation: number; // ラジアン
};

export type SelectionType = "rectangle" | "lasso" | "magic";

export type UserInput =
  | {
      type: "DrawStroke";
      payload: {
        points: Point[];
        color: string;
        width: number;
        layer_id: string;
      };
    }
  | {
      type: "ChangeTool";
      payload: {
        tool_id: string;
      };
    }
  | {
      type: "CreateLayer";
      payload: {
        name: string;
      };
    }
  | {
      type: "DeleteLayer";
      payload: {
        layer_id: string;
      };
    }
  | {
      type: "ReorderLayer";
      payload: {
        layer_id: string;
        new_index: number;
      };
    }
  | {
      type: "ChangeLayerOpacity";
      payload: {
        layer_id: string;
        opacity: number;
      };
    }
  | {
      type: "ChangeLayerBlendMode";
      payload: {
        layer_id: string;
        blend_mode: string;
      };
    }
  | {
      type: "Fill";
      payload: {
        point: Point;
        color: string;
        layer_id: string;
      };
    }
  | {
      type: "CreateSelection";
      payload: {
        selection_type: SelectionType;
        points: Point[];
      };
    }
  | {
      type: "TransformSelection";
      payload: {
        transform: Transform;
      };
    }
  | {
      type: "Undo";
      payload: {};
    }
  | {
      type: "Redo";
      payload: {};
    };

// ============================
// DrawCommand（バックエンド→フロントエンド）
// ============================

export type Rect = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export type DrawCommand =
  | {
      type: "ClearCanvas";
      payload: {};
    }
  | {
      type: "DrawPath";
      payload: {
        points: Point[];
        color: string;
        width: number;
        layer_id: string;
      };
    }
  | {
      type: "UpdateRasterArea";
      payload: {
        rect: Rect;
        pixel_data: number[];
        layer_id: string;
      };
    }
  | {
      type: "AddLayer";
      payload: {
        layer_id: string;
        index: number;
      };
    }
  | {
      type: "RemoveLayer";
      payload: {
        layer_id: string;
      };
    }
  | {
      type: "ReorderLayers";
      payload: {
        layer_ids: string[];
      };
    }
  | {
      type: "UpdateLayerProperties";
      payload: {
        layer_id: string;
        opacity: number;
        blend_mode: string;
        visible: boolean;
      };
    }
  | {
      type: "ShowSelection";
      payload: {
        selection_type: SelectionType;
        points: Point[];
      };
    }
  | {
      type: "ClearSelection";
      payload: {};
    }
  | {
      type: "ApplyTransform";
      payload: {
        layer_id: string;
        transform: Transform;
      };
    }
  | {
      type: "Batch";
      payload: {
        commands: DrawCommand[];
      };
    };

// ============================
// DrawingState（現在の状態）
// ============================

export interface HybridLayerInfo {
  id: string;
  name: string;
  visible: boolean;
  opacity: number;
  blend_mode: string;
}

export interface DrawingStateInfo {
  layers: HybridLayerInfo[];
  active_layer_id: string | null;
  current_tool: string;
  current_color: string;
  current_brush_size: number;
}