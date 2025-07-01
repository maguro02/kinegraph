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

// アトム定義
export const projectAtom = atom<Project | null>(null);
export const currentFrameIndexAtom = atom<number>(0);
export const selectedLayerAtom = atom<string | null>(null);
export const toolAtom = atom<'pen' | 'eraser' | 'bucket' | 'select'>('pen');
export const brushSizeAtom = atom<number>(5);
export const colorAtom = atom<string>('#000000');
export const jointTypeAtom = atom<JointType>({ type: 'round', segments: 8 });

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