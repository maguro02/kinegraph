// publicディレクトリのWASMモジュールを使用
const WASM_PATH = '/wasm/kinegraph_wasm.js';

export interface DrawingContextInterface {
  draw_stroke(points: Float32Array, color: Float32Array, width: number): void;
  clear(color: Float32Array): void;
  get_pixels(): Uint8Array;
  set_shared_buffer(buffer: SharedBufferInterface): void;
  copy_to_shared_buffer(): void;
  resize(width: number, height: number): void;
  free?(): void;
}

export interface SharedBufferInterface {
  buffer(): ArrayBuffer;
  free(): void;
}

export interface WasmExports {
  check_webgpu_support: () => Promise<boolean>;
  DrawingContext: new (width: number, height: number) => Promise<DrawingContextInterface>;
  SharedBuffer: SharedBufferInterface;
  checkRequiredFeatures?: () => {
    webgpu: boolean;
    offscreenCanvas: boolean;
    sharedArrayBuffer: boolean;
    crossOriginIsolated: boolean;
  };
}

let wasmModule: WasmExports | null = null;
let initPromise: Promise<WasmExports> | null = null;

export async function initWasm(): Promise<WasmExports> {
  if (wasmModule) {
    return wasmModule;
  }

  if (initPromise) {
    return initPromise;
  }

  initPromise = (async () => {
    try {
      console.log('Loading WASM module from:', WASM_PATH);
      
      // publicディレクトリのWASMモジュールを動的インポート
      const wasmModuleUrl = new URL(WASM_PATH, window.location.origin).href;
      const module = await import(/* @vite-ignore */ wasmModuleUrl);
      
      // 初期化関数を呼び出す（wasm-bindgenのデフォルトエクスポート）
      if (module.default) {
        await module.default();
      }
      
      // エクスポートされた関数を確認
      console.log('WASM module exports:', Object.keys(module));
      
      wasmModule = {
        check_webgpu_support: module.check_webgpu_support || (async () => {
          if (!navigator.gpu) {
            return false;
          }
          try {
            const adapter = await navigator.gpu.requestAdapter();
            return adapter !== null;
          } catch {
            return false;
          }
        }),
        DrawingContext: module.DrawingContext || class MockDrawingContext {
          static async new(width: number, height: number): Promise<DrawingContextInterface> {
            console.warn('DrawingContext not available in WASM exports - using mock');
            return {
              draw_stroke(_points: Float32Array, _color: Float32Array, _width: number) {
                console.warn('Mock draw_stroke called');
              },
              clear(_color: Float32Array) {
                console.warn('Mock clear called');
              },
              get_pixels() { 
                console.warn('Mock get_pixels called');
                return new Uint8Array(width * height * 4); 
              },
              set_shared_buffer(_buffer: SharedBufferInterface) {
                console.warn('Mock set_shared_buffer called');
              },
              copy_to_shared_buffer() {
                console.warn('Mock copy_to_shared_buffer called');
              },
              resize(_width: number, _height: number) {
                console.warn('Mock resize called');
              }
            };
          }
          constructor(width: number, height: number) {
            console.log(`MockDrawingContext constructor called with ${width}x${height}`);
            // Return the static new method result for compatibility
            return (this.constructor as any).new(width, height);
          }
        } as any,
        SharedBuffer: module.SharedBuffer || {
          buffer() { return new ArrayBuffer(0); },
          free() {}
        },
        checkRequiredFeatures: module.checkRequiredFeatures
      };
      
      // 必要な機能をチェック
      if (wasmModule.checkRequiredFeatures) {
        const features = wasmModule.checkRequiredFeatures();
        console.log('Required features check:', features);
      }
      
      console.log('WASM module loaded successfully');
      return wasmModule;
    } catch (error) {
      console.error('Failed to initialize WASM:', error);
      throw error;
    }
  })();

  return initPromise;
}

export function getWasmModule(): WasmExports | null {
  return wasmModule;
}