/**
 * Tauri IPC TypeScript型定義
 * Rustバックエンドとの通信に使用される型定義をまとめたファイル
 */

// ============================
// 基本的なデータ型
// ============================

/**
 * 2D座標を表す型
 */
export interface Point {
  x: number;
  y: number;
}

/**
 * 矩形領域を表す型
 */
export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * 変形情報を表す型
 */
export interface Transform {
  translate_x: number;
  translate_y: number;
  scale_x: number;
  scale_y: number;
  rotation: number; // ラジアン
}

/**
 * 選択範囲のタイプ
 */
export type SelectionType = "rectangle" | "lasso" | "magic";

/**
 * ブレンドモードの種類
 */
export type BlendMode = "Normal" | "Multiply" | "Screen" | "Overlay";

/**
 * ツールの種類
 */
export type ToolType = "pen" | "eraser" | "bucket" | "select";

// ============================
// レイヤー関連の型
// ============================

/**
 * レイヤー情報（Tauri API用）
 */
export interface LayerInfo {
  id: string;
  name: string;
  visible: boolean;
  opacity: number;
  blend_mode: string;
}

/**
 * レイヤー情報（フロントエンド用）
 */
export interface Layer {
  id: string;
  name: string;
  visible: boolean;
  opacity: number;
  blendMode: BlendMode;
  locked: boolean;
}

// ============================
// プロジェクト関連の型
// ============================

/**
 * フレーム情報
 */
export interface Frame {
  id: string;
  layers: Layer[];
  duration: number;
}

/**
 * プロジェクト情報
 */
export interface Project {
  id: string;
  name: string;
  width: number;
  height: number;
  frameRate: number;
  frames: Frame[];
}

// ============================
// 描画エンジンの状態
// ============================

/**
 * 描画エンジンの現在の状態
 */
export interface DrawingStateInfo {
  layers: LayerInfo[];
  active_layer_id: string | null;
  current_tool: string;
  current_color: string;
  current_brush_size: number;
}

// ============================
// ブラシ設定
// ============================

/**
 * ブラシの設定情報
 */
export interface BrushSettings {
  size: number;
  opacity: number;
  color: string;
  hardness: number;
  spacing: number;
}

// ============================
// ユーザー入力コマンド（フロントエンド→バックエンド）
// ============================

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
// 描画コマンド（バックエンド→フロントエンド）
// ============================

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
// APIレスポンス型
// ============================

/**
 * レイヤー作成のレスポンス
 */
export interface CreateLayerResponse {
  status: string;
  layer_id: string;
  commands: DrawCommand[];
}

/**
 * レイヤー削除のレスポンス
 */
export interface RemoveLayerResponse {
  status: string;
  removed_layer_id: string;
  commands: DrawCommand[];
}

// ============================
// ジョイントタイプ（パス描画用）
// ============================

export type JointType = 
  | { type: 'miter'; limit: number }
  | { type: 'round'; segments: number }
  | { type: 'bevel' };

// ============================
// エラー型
// ============================

/**
 * Tauri APIエラー
 */
export interface TauriError {
  message: string;
  code?: string;
}

// ============================
// ユーティリティ型
// ============================

/**
 * IDを持つオブジェクトの基本型
 */
export interface Identifiable {
  id: string;
}

/**
 * タイムスタンプ付きのオブジェクト
 */
export interface Timestamped {
  createdAt: number;
  updatedAt: number;
}

// ============================
// 型ガード関数
// ============================

/**
 * UserInput型のタイプガード
 */
export function isUserInput(value: unknown): value is UserInput {
  if (!value || typeof value !== 'object') return false;
  const obj = value as any;
  return 'type' in obj && 'payload' in obj;
}

/**
 * DrawCommand型のタイプガード
 */
export function isDrawCommand(value: unknown): value is DrawCommand {
  if (!value || typeof value !== 'object') return false;
  const obj = value as any;
  return 'type' in obj && 'payload' in obj;
}

// ============================
// デフォルト値
// ============================

/**
 * デフォルトのTransform
 */
export const defaultTransform: Transform = {
  translate_x: 0,
  translate_y: 0,
  scale_x: 1,
  scale_y: 1,
  rotation: 0,
};

/**
 * デフォルトのBrushSettings
 */
export const defaultBrushSettings: BrushSettings = {
  size: 5,
  opacity: 1,
  color: '#000000',
  hardness: 0.8,
  spacing: 0.1,
};