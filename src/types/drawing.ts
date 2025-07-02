// 描画関連の型定義

// 座標点
export interface Point {
  x: number;
  y: number;
  pressure?: number;
}

// 色
export interface Color {
  r: number; // 0-255
  g: number; // 0-255
  b: number; // 0-255
  a: number; // 0.0-1.0
}

// ブラシツールの種類
export type BrushType = 'pen' | 'pencil' | 'brush' | 'eraser' | 'marker';

// ブラシツール設定
export interface BrushTool {
  type: BrushType;
  size: number;
  opacity: number;
  smoothing: number;
  pressure: boolean;
}

// ストロークID
export type StrokeId = string;

// 描画コンテキストインターフェース
export interface DrawingContext {
  // 初期化
  initialize(): Promise<void>;
  
  // ストローク操作
  startStroke(point: Point, tool: BrushTool, color: Color): Promise<StrokeId>;
  addPoint(strokeId: StrokeId, point: Point): Promise<void>;
  endStroke(strokeId: StrokeId): Promise<void>;
  
  // キャンバス操作
  clear(): Promise<void>;
  resize(width: number, height: number): void;
  
  // クリーンアップ
  destroy(): void;
}

// レイヤー
export interface Layer {
  id: string;
  name: string;
  visible: boolean;
  opacity: number;
  blendMode: BlendMode;
  locked: boolean;
}

// ブレンドモード
export type BlendMode = 'normal' | 'multiply' | 'screen' | 'overlay' | 'soft-light' | 'hard-light' | 'color-dodge' | 'color-burn' | 'darken' | 'lighten' | 'difference' | 'exclusion';

// フレーム
export interface Frame {
  id: string;
  layerData: Map<string, ImageData>;
  duration: number; // milliseconds
}

// アニメーションプロジェクト
export interface AnimationProject {
  id: string;
  name: string;
  width: number;
  height: number;
  fps: number;
  frames: Frame[];
  layers: Layer[];
  currentFrameIndex: number;
  currentLayerId: string;
}