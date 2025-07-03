import { atom } from 'jotai';

// プロジェクト状態管理
export interface Project {
  id: string;
  name: string;
  width: number;
  height: number;
  frameRate: number;
  frames: Frame[];
}

export interface Frame {
  id: string;
  layers: Layer[];
  duration: number;
}

export interface Layer {
  id: string;
  name: string;
  visible: boolean;
  opacity: number;
  blendMode: 'Normal' | 'Multiply' | 'Screen' | 'Overlay';
  locked: boolean;
}

// ジョイントタイプ
export type JointType = 
  | { type: 'miter'; limit: number }
  | { type: 'round'; segments: number }
  | { type: 'bevel' };

// ブラシ設定
export interface BrushSettings {
  size: number;
  opacity: number;
  color: string;
  hardness: number;
  spacing: number;
}

// アトム定義
export const projectAtom = atom<Project | null>(null);
export const currentFrameIndexAtom = atom<number>(0);
export const selectedLayerAtom = atom<string | null>(null);
export const toolAtom = atom<'pen' | 'eraser' | 'bucket' | 'select'>('pen');
export const activeToolAtom = toolAtom; // エイリアスとして追加
export const brushSizeAtom = atom<number>(5);
export const colorAtom = atom<string>('#000000');
export const jointTypeAtom = atom<JointType>({ type: 'round', segments: 8 });

// 新しいアトム
export const currentLayerIdAtom = atom<string>('layer-1');
export const activeLayerIdAtom = atom<string>('default-layer');
export const brushSettingsAtom = atom<BrushSettings>({
  size: 5,
  opacity: 1,
  color: '#000000',
  hardness: 0.8,
  spacing: 0.1,
});

// 描画エンジンの種類
export type DrawingEngineType = 'canvas2d' | 'wasm' | 'wasmWorker' | 'tauri';

// 描画エンジン選択アトム
export const drawingEngineAtom = atom<DrawingEngineType>('tauri');

// Tauri描画エンジンの状態
export interface DrawingEngineState {
  canvasId: string | null;
  isInitialized: boolean;
  error: string | null;
}

// Tauri描画エンジンの状態アトム
export const drawingEngineStateAtom = atom<DrawingEngineState>({
  canvasId: null,
  isInitialized: false,
  error: null,
});

// 計算されたアトム
export const currentFrameAtom = atom((get) => {
  const project = get(projectAtom);
  const index = get(currentFrameIndexAtom);
  return project?.frames[index] || null;
});

export const selectedLayerDataAtom = atom((get) => {
  const frame = get(currentFrameAtom);
  const selectedId = get(selectedLayerAtom);
  return frame?.layers.find(layer => layer.id === selectedId) || null;
});