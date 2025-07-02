import type { DrawingContext, Point, StrokeId, Color, BrushTool } from '../types/drawing.ts';
import type { DrawRequest, DrawResponse } from '../types/drawEngine.ts';

/**
 * WebAssembly描画エンジンのメインスレッド側実装
 * Web Worker内のDrawEngineと通信し、SharedArrayBuffer経由でCanvasに描画
 */
export class WasmDrawEngine implements DrawingContext {
  private worker: Worker | null = null;
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private width: number;
  private height: number;
  private canvasBuffer: SharedArrayBuffer | null = null;
  private canvasView: Uint8ClampedArray | null = null;
  private imageData: ImageData | null = null;
  
  // Promise管理用
  private pendingRequests = new Map<string, {
    resolve: (value: any) => void;
    reject: (error: Error) => void;
  }>();
  private requestIdCounter = 0;
  
  // ストロークID管理
  private strokeIdCounter = 0;

  constructor(canvas: HTMLCanvasElement, width: number, height: number) {
    this.canvas = canvas;
    this.width = width;
    this.height = height;

    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    if (!ctx) {
      throw new Error('Failed to get 2D context');
    }
    this.ctx = ctx;

    // キャンバスサイズ設定
    canvas.width = width;
    canvas.height = height;
    
    // ImageDataを初期化
    this.imageData = new ImageData(width, height);
  }

  async initialize(): Promise<void> {
    // Workerを起動
    this.worker = new Worker(
      new URL('./wasmDrawEngineWorker.ts', import.meta.url),
      { type: 'module' }
    );
    
    // メッセージハンドラを設定
    this.worker.addEventListener('message', this.handleWorkerMessage.bind(this));
    
    // Workerを初期化
    const result = await this.sendRequest('init', {
      width: this.width,
      height: this.height
    });
    
    // WorkerからSharedArrayBufferを取得
    if (result && result.sharedBuffer) {
      this.canvasBuffer = result.sharedBuffer;
      console.log('Received SharedArrayBuffer from Worker');
    } else {
      // フォールバック
      console.warn('SharedArrayBuffer not available from Worker, creating local buffer');
      const bufferSize = this.width * this.height * 4; // RGBA
      this.canvasBuffer = new SharedArrayBuffer(bufferSize);
    }
    
    // nullチェック後にUint8ClampedArrayを作成
    if (this.canvasBuffer) {
      this.canvasView = new Uint8ClampedArray(this.canvasBuffer);
    }
    
    // ImageDataを作成
    this.imageData = new ImageData(this.width, this.height);
  }

  async startStroke(point: Point, tool: BrushTool, color: Color): Promise<StrokeId> {
    const strokeId = (this.strokeIdCounter++).toString();
    
    await this.sendRequest('startStroke', {
      point,
      tool,
      color,
      strokeId
    });
    
    return strokeId;
  }

  async addPoint(strokeId: StrokeId, point: Point): Promise<void> {
    await this.sendRequest('addPoint', {
      strokeId,
      point
    });
  }

  async endStroke(strokeId: StrokeId): Promise<void> {
    await this.sendRequest('endStroke', {
      strokeId
    });
  }

  async clear(): Promise<void> {
    await this.sendRequest('clear', {});
  }

  resize(width: number, height: number): void {
    // キャンバスサイズを更新
    this.width = width;
    this.height = height;
    this.canvas.width = width;
    this.canvas.height = height;
    
    // Workerに通知（非同期だが結果を待たない）
    this.sendRequest('resize', {
      width,
      height
    }).then(() => {
      // Worker側で新しいSharedArrayBufferが作られた後に更新
      const bufferSize = width * height * 4;
      this.canvasBuffer = new SharedArrayBuffer(bufferSize);
      this.canvasView = new Uint8ClampedArray(this.canvasBuffer);
      this.imageData = new ImageData(width, height);
    }).catch(console.error);
  }

  destroy(): void {
    if (this.worker) {
      // Workerに終了を通知
      this.sendRequest('destroy', {}).finally(() => {
        this.worker?.terminate();
        this.worker = null;
      });
    }
    
    this.canvasBuffer = null;
    this.canvasView = null;
    this.pendingRequests.clear();
  }

  private async sendRequest(type: DrawRequest['type'], data: any): Promise<any> {
    if (!this.worker) {
      throw new Error('Worker not initialized');
    }
    
    const requestId = `${type}_${this.requestIdCounter++}`;
    
    return new Promise((resolve, reject) => {
      this.pendingRequests.set(requestId, { resolve, reject });
      
      const request: DrawRequest = {
        type,
        data,
        requestId
      };
      
      this.worker!.postMessage(request);
      
      // タイムアウト設定（5秒）
      setTimeout(() => {
        if (this.pendingRequests.has(requestId)) {
          this.pendingRequests.delete(requestId);
          reject(new Error(`Request ${type} timed out`));
        }
      }, 5000);
    });
  }

  private handleWorkerMessage(event: MessageEvent<DrawResponse>) {
    const { type, data } = event.data;
    
    // レンダリング通知の場合
    if (type === 'rendered') {
      this.updateCanvas();
      return;
    }
    
    // エラーレスポンスの場合
    if (type === 'error') {
      console.error('Worker error:', data.message);
      // 該当するリクエストを探してreject
      for (const [requestId, promise] of this.pendingRequests) {
        if (requestId.startsWith(data.originalType)) {
          promise.reject(new Error(data.message));
          this.pendingRequests.delete(requestId);
          break;
        }
      }
      return;
    }
    
    // 通常のレスポンス処理
    const responseTypeMap: Record<DrawResponse['type'], string> = {
      'initialized': 'init',
      'strokeStarted': 'startStroke',
      'pointAdded': 'addPoint',
      'strokeEnded': 'endStroke',
      'cleared': 'clear',
      'resized': 'resize',
      'destroyed': 'destroy',
      'rendered': '',
      'error': ''
    };
    
    const originalType = responseTypeMap[type];
    if (originalType) {
      // 該当するリクエストを探してresolve
      for (const [requestId, promise] of this.pendingRequests) {
        if (requestId.startsWith(originalType)) {
          promise.resolve(data);
          this.pendingRequests.delete(requestId);
          break;
        }
      }
    }
  }

  private updateCanvas(): void {
    if (!this.canvasView || !this.imageData || !this.canvasBuffer) {
      return;
    }
    
    // SharedArrayBufferからImageDataにデータをコピー
    this.imageData.data.set(this.canvasView);
    
    // Canvasに描画
    this.ctx.putImageData(this.imageData, 0, 0);
  }
}

/**
 * WebAssembly描画エンジンを作成するファクトリ関数
 */
export function createWasmDrawEngine(
  canvas: HTMLCanvasElement,
  width: number,
  height: number
): DrawingContext {
  return new WasmDrawEngine(canvas, width, height);
}