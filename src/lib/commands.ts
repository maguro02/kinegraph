/**
 * Tauri IPCコマンドの定義
 * tauri-spectaによって生成された型安全なバインディングを使用
 */

// tauri-spectaで生成されたバインディングをインポート
export { commands } from './bindings';

// 生成された型もエクスポート（必要に応じて）
export type {
    CreateLayerResponse,
    DrawCommand,
    DrawingStateInfo,
    HybridLayerInfo,
    Point,
    Rect,
    RemoveLayerResponse,
    SelectionType,
    Transform,
    UserInput,
} from './bindings';