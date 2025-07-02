import type { DrawingContext, Point, StrokeId, Color, BrushTool } from '../types/drawing.ts';

// Worker通信用のメッセージタイプ
interface WorkerMessage {
  id: string;
  type: string;
  [key: string]: any;
}

interface WorkerResponse {
  id: string;
  type: string;
  data?: any;
  error?: string;
}

export class WasmCanvasDrawingContext implements DrawingContext {
  private worker: Worker | null = null;
  private messageId = 0;
  private pendingMessages = new Map<string, { resolve: Function; reject: Function }>();
  private initialized = false;

  constructor(
    private canvas: HTMLCanvasElement,
    private width: number,
    private height: number
  ) {}

  async initialize(): Promise<void> {
    if (this.initialized) return;

    try {
      // OffscreenCanvasを作成
      const offscreenCanvas = this.canvas.transferControlToOffscreen();

      // Workerを作成
      this.worker = new Worker(
        new URL('./wasmCanvasWorker.ts', import.meta.url),
        { type: 'module' }
      );

      // Workerメッセージハンドラを設定
      this.worker.addEventListener('message', (event: MessageEvent<WorkerResponse>) => {
        const response = event.data;
        const pending = this.pendingMessages.get(response.id);
        
        if (pending) {
          this.pendingMessages.delete(response.id);
          if (response.error) {
            pending.reject(new Error(response.error));
          } else {
            pending.resolve(response.data);
          }
        }
      });

      // Workerを初期化
      // publicディレクトリからのWASMファイルパス
      const wasmUrl = new URL('/wasm/kinegraph_wasm_bg.wasm', window.location.origin).href;
      
      console.log('Initializing WASM worker with:', { wasmUrl, width: this.width, height: this.height });
      
      const initResult = await this.sendMessage({
        type: 'init',
        canvas: offscreenCanvas,
        wasmUrl,
        width: this.width,
        height: this.height
      }, [offscreenCanvas]);
      
      console.log('WASM worker initialization result:', initResult);

      this.initialized = true;
    } catch (error) {
      console.error('Failed to initialize WASM canvas:', error);
      throw error;
    }
  }

  private async sendMessage(message: Omit<WorkerMessage, 'id'> & { type: string }, transfer?: Transferable[]): Promise<any> {
    if (!this.worker) {
      throw new Error('Worker not initialized');
    }

    const id = (this.messageId++).toString();
    const fullMessage: WorkerMessage = { ...message, id };

    return new Promise((resolve, reject) => {
      this.pendingMessages.set(id, { resolve, reject });
      
      if (transfer) {
        this.worker!.postMessage(fullMessage, transfer);
      } else {
        this.worker!.postMessage(fullMessage);
      }
    });
  }

  async startStroke(point: Point, tool: BrushTool, color: Color): Promise<StrokeId> {
    if (!this.initialized) {
      throw new Error('WasmCanvas not initialized');
    }
    
    console.log('Starting stroke with:', { point, tool, color });
    
    const result = await this.sendMessage({
      type: 'startStroke',
      point,
      tool,
      color
    });
    
    console.log('Stroke started with ID:', result.strokeId);
    
    // 最初のポイントの描画はスキップ（2点目を待つ）
    // await this.updateCanvas();
    return result.strokeId;
  }

  async addPoint(strokeId: StrokeId, point: Point): Promise<void> {
    if (!this.initialized) {
      throw new Error('WasmCanvas not initialized');
    }
    
    await this.sendMessage({
      type: 'addPoint',
      strokeId,
      point
    });
    
    // ポイント追加後にキャンバスを更新
    await this.updateCanvas();
  }

  async endStroke(strokeId: StrokeId): Promise<void> {
    await this.sendMessage({
      type: 'endStroke',
      strokeId
    });
    
    await this.updateCanvas();
  }

  async clear(): Promise<void> {
    await this.sendMessage({
      type: 'clear'
    });
    
    await this.updateCanvas();
  }

  private async updateCanvas(): Promise<void> {
    await this.sendMessage({
      type: 'updateCanvas'
    });
  }

  resize(width: number, height: number): void {
    this.width = width;
    this.height = height;
    // リサイズ処理はWorker側で実装する必要がある
    console.warn('Canvas resize not yet implemented for WASM context');
  }

  destroy(): void {
    if (this.worker) {
      this.worker.terminate();
      this.worker = null;
    }
    this.pendingMessages.clear();
    this.initialized = false;
  }
}

// WASMキャンバスコンテキストを作成するファクトリ関数
export async function createWasmCanvasContext(
  canvas: HTMLCanvasElement,
  width: number,
  height: number
): Promise<DrawingContext> {
  const context = new WasmCanvasDrawingContext(canvas, width, height);
  await context.initialize();
  return context;
}

// WebGPUサポートをチェックする関数
export async function checkWebGPUSupport(): Promise<boolean> {
  try {
    // navigator.gpuの存在をチェック
    if (!('gpu' in navigator)) {
      return false;
    }
    
    // GPUアダプターを取得できるか確認
    const adapter = await (navigator as any).gpu.requestAdapter();
    return adapter !== null;
  } catch (error) {
    console.error('Failed to check WebGPU support:', error);
    return false;
  }
}