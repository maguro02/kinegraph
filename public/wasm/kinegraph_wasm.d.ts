/* tslint:disable */
/* eslint-disable */
export function setup_worker(scope: DedicatedWorkerGlobalScope): void;
export function main(): void;
export function check_webgpu_support(): boolean;

export interface DrawingPoint {
    x: number;
    y: number;
    pressure: number;
}

export interface StrokeData {
    points: Float32Array;
    color: [number, number, number, number];
    width: number;
}

export interface DrawingOptions {
    width: number;
    height: number;
    useSharedBuffer?: boolean;
}

export interface RenderStats {
    drawCalls: number;
    vertices: number;
    frameTime: number;
}


/**
 * WebGPU/wgpuベースの描画エンジン
 */
export class DrawEngine {
  free(): void;
  /**
   * 新しいDrawEngineインスタンスを作成 (WASM API)
   */
  constructor(canvas_width: number, canvas_height: number);
  /**
   * キャンバスサイズを更新
   */
  resize(new_width: number, new_height: number): void;
  /**
   * 新しいストロークを開始 (WASM API)
   */
  begin_stroke(x: number, y: number, pressure: number, r: number, g: number, b: number, a: number, brush_type: number, size: number): number;
  /**
   * アクティブなストロークにポイントを追加 (WASM API)
   */
  add_point(x: number, y: number, pressure: number): void;
  /**
   * ストロークを終了し、保存 (WASM API)
   */
  end_stroke(): void;
  /**
   * ストロークを削除 (WASM API)
   */
  remove_stroke(stroke_id: number): boolean;
  /**
   * 全ストロークをクリア (WASM API)
   */
  clear(): void;
  /**
   * アンドゥ操作 (WASM API)
   */
  undo(): boolean;
  /**
   * リドゥ操作 (WASM API)
   */
  redo(): boolean;
  /**
   * SharedArrayBufferを取得 (WASM API)
   */
  get_shared_buffer(): SharedArrayBuffer;
  /**
   * 描画を実行してSharedArrayBufferに書き込み
   */
  render(): void;
}
export class DrawingContext {
  private constructor();
  free(): void;
  static new(width: number, height: number): Promise<DrawingContext>;
  draw_stroke(points: Float32Array, color: Float32Array, width: number): void;
  clear(color: Float32Array): void;
  get_pixels(): Uint8Array;
  set_shared_buffer(buffer: SharedBuffer): void;
  copy_to_shared_buffer(): void;
  draw_stroke_from_typed_array(points: Float32Array, color: Float32Array, width: number): void;
  resize(width: number, height: number): void;
}
export class SharedBuffer {
  free(): void;
  constructor(size: number);
  write_pixels(offset: number, pixels: Uint8Array): void;
  readonly buffer: SharedArrayBuffer;
}
export class WebGPURenderer {
  private constructor();
  free(): void;
  static new(canvas?: OffscreenCanvas | null): Promise<WebGPURenderer>;
  render(): void;
  resize(width: number, height: number): void;
}
export class WorkerContext {
  private constructor();
  free(): void;
  init(canvas: OffscreenCanvas): Promise<void>;
  process_draw_command(command: any): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_workercontext_free: (a: number, b: number) => void;
  readonly workercontext_init: (a: number, b: number) => number;
  readonly workercontext_process_draw_command: (a: number, b: number, c: number) => void;
  readonly setup_worker: (a: number, b: number) => void;
  readonly __wbg_drawengine_free: (a: number, b: number) => void;
  readonly drawengine_new: (a: number, b: number, c: number) => void;
  readonly drawengine_resize: (a: number, b: number, c: number) => void;
  readonly drawengine_begin_stroke: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => void;
  readonly drawengine_add_point: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly drawengine_end_stroke: (a: number, b: number) => void;
  readonly drawengine_remove_stroke: (a: number, b: number) => number;
  readonly drawengine_clear: (a: number, b: number) => void;
  readonly drawengine_undo: (a: number, b: number) => void;
  readonly drawengine_redo: (a: number, b: number) => void;
  readonly drawengine_get_shared_buffer: (a: number) => number;
  readonly drawengine_render: (a: number, b: number) => void;
  readonly __wbg_webgpurenderer_free: (a: number, b: number) => void;
  readonly webgpurenderer_new: (a: number) => number;
  readonly webgpurenderer_render: (a: number, b: number) => void;
  readonly webgpurenderer_resize: (a: number, b: number, c: number) => void;
  readonly __wbg_sharedbuffer_free: (a: number, b: number) => void;
  readonly sharedbuffer_new: (a: number, b: number) => void;
  readonly sharedbuffer_buffer: (a: number) => number;
  readonly sharedbuffer_write_pixels: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly __wbg_drawingcontext_free: (a: number, b: number) => void;
  readonly drawingcontext_new: (a: number, b: number) => number;
  readonly drawingcontext_draw_stroke: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
  readonly drawingcontext_clear: (a: number, b: number, c: number, d: number) => void;
  readonly drawingcontext_get_pixels: (a: number, b: number) => void;
  readonly drawingcontext_set_shared_buffer: (a: number, b: number) => void;
  readonly drawingcontext_copy_to_shared_buffer: (a: number, b: number) => void;
  readonly drawingcontext_draw_stroke_from_typed_array: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly drawingcontext_resize: (a: number, b: number, c: number, d: number) => void;
  readonly main: () => void;
  readonly check_webgpu_support: () => number;
  readonly __wbindgen_export_0: (a: number) => void;
  readonly __wbindgen_export_1: (a: number, b: number, c: number) => void;
  readonly __wbindgen_export_2: (a: number, b: number) => number;
  readonly __wbindgen_export_3: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export_4: WebAssembly.Table;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
  readonly __wbindgen_export_5: (a: number, b: number, c: number) => void;
  readonly __wbindgen_export_6: (a: number, b: number, c: number, d: number) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
