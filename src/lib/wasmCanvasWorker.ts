/// <reference lib="webworker" />

import type { Point, StrokeId, Color, BrushTool } from '../types/drawing.ts';

// WASMモジュールを動的インポート
let wasmModule: any = null;
let wasmInitialized = false;

// メッセージタイプの定義
interface WorkerMessage {
  type: 'init' | 'startStroke' | 'addPoint' | 'endStroke' | 'clear' | 'updateCanvas';
  id: string;
}

interface InitMessage extends WorkerMessage {
  type: 'init';
  canvas: OffscreenCanvas;
  wasmUrl: string;
  width: number;
  height: number;
}

interface StartStrokeMessage extends WorkerMessage {
  type: 'startStroke';
  point: Point;
  tool: BrushTool;
  color: Color;
}

interface AddPointMessage extends WorkerMessage {
  type: 'addPoint';
  strokeId: StrokeId;
  point: Point;
}

interface EndStrokeMessage extends WorkerMessage {
  type: 'endStroke';
  strokeId: StrokeId;
}

interface ClearMessage extends WorkerMessage {
  type: 'clear';
}

interface UpdateCanvasMessage extends WorkerMessage {
  type: 'updateCanvas';
}

type IncomingMessage = InitMessage | StartStrokeMessage | AddPointMessage | EndStrokeMessage | ClearMessage | UpdateCanvasMessage;

// レスポンスタイプの定義
interface WorkerResponse {
  type: 'initialized' | 'strokeStarted' | 'pointAdded' | 'strokeEnded' | 'cleared' | 'canvasUpdated' | 'error';
  id: string;
  data?: any;
  error?: string;
}

// ストロークデータを管理するクラス
interface StrokeData {
  points: number[]; // [x, y, x, y, ...]形式で座標を保存
  tool: BrushTool;
  color: Color;
}

// Workerコンテキストクラス
class WorkerContext {
  private ctx: OffscreenCanvasRenderingContext2D | null = null;
  private drawingContext: any | null = null; // DrawingContextのインスタンス
  private sharedBuffer: any | null = null; // SharedBufferのインスタンス
  private imageData: ImageData | null = null;
  private activeStrokes: Map<StrokeId, StrokeData> = new Map();
  private strokeIdCounter: number = 0;

  async initialize(canvas: OffscreenCanvas, wasmUrl: string, width: number, height: number): Promise<void> {

    // OffscreenCanvasの2Dコンテキストを取得
    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D context from OffscreenCanvas');
    }
    this.ctx = ctx;

    // キャンバスサイズを設定
    canvas.width = width;
    canvas.height = height;

    // ImageDataを作成
    this.imageData = ctx.createImageData(width, height);

    // WASMモジュールを初期化
    try {
      if (!wasmInitialized) {
        // WASMモジュールをimport()で動的にロード
        const jsUrl = new URL('/wasm/kinegraph_wasm.js', self.location.origin).href;
        console.log('Importing WASM module from:', jsUrl);
        
        try {
          // 動的インポートを使用
          wasmModule = await import(jsUrl);
          console.log('WASM module imported successfully:', Object.keys(wasmModule));
        } catch (importError) {
          console.error('Failed to import WASM module:', importError);
          throw new Error(`Failed to import WASM module: ${importError}`);
        }
        
        if (!wasmModule || !wasmModule.default) {
          throw new Error('WASM module does not have default export');
        }
        
        console.log('Initializing WASM binary from:', wasmUrl);
        // WASMバイナリを初期化
        await wasmModule.default(wasmUrl);
        wasmInitialized = true;
        console.log('WASM initialized successfully');
      }
      
      if (!wasmModule) {
        throw new Error('WASM module not loaded');
      }
      
      const { DrawingContext, SharedBuffer, check_webgpu_support } = wasmModule;
      
      // WebGPUサポートをチェック
      const hasWebGPU = check_webgpu_support();
      console.log('WebGPU support:', hasWebGPU);
      
      // DrawingContextを作成
      console.log('Creating DrawingContext with dimensions:', width, 'x', height);
      try {
        // DrawingContextはwasm-bindgenでasync newメソッドを持つため、
        // JavaScript側ではコンストラクターとして呼び出す
        console.log('Using DrawingContext constructor');
        this.drawingContext = await new DrawingContext(width, height);
        
        // DrawingContextが正しく作成されたか確認
        if (!this.drawingContext || typeof this.drawingContext.draw_stroke !== 'function') {
          throw new Error('DrawingContext was not properly initialized');
        }
        
        console.log('DrawingContext created successfully:', {
          hasDrawStroke: typeof this.drawingContext.draw_stroke === 'function',
          hasClear: typeof this.drawingContext.clear === 'function',
          hasGetPixels: typeof this.drawingContext.get_pixels === 'function'
        });
      } catch (error) {
        console.error('Failed to create DrawingContext:', error);
        throw new Error(`DrawingContext creation failed: ${error}`);
      }
      
      // SharedArrayBufferが利用可能な場合は、SharedBufferを作成
      if (typeof SharedArrayBuffer !== 'undefined') {
        try {
          this.sharedBuffer = new SharedBuffer(width * height * 4);
          this.drawingContext.set_shared_buffer(this.sharedBuffer);
          console.log('SharedArrayBuffer enabled for high-performance rendering');
        } catch (e) {
          console.warn('Failed to create SharedBuffer:', e);
        }
      }
      
      // 初期クリア（白背景）
      this.drawingContext.clear(new Float32Array([1.0, 1.0, 1.0, 1.0]));
      
    } catch (error) {
      console.error('WASM initialization error:', error);
      throw new Error(`Failed to initialize WASM module: ${error}`);
    }
  }

  startStroke(point: Point, tool: BrushTool, color: Color): StrokeId {
    if (!this.drawingContext) {
      throw new Error('Worker context not initialized');
    }

    // 新しいストロークIDを生成
    const strokeId = (this.strokeIdCounter++).toString();
    
    // ストロークデータを初期化
    const strokeData: StrokeData = {
      points: [point.x, point.y],
      tool,
      color
    };
    
    this.activeStrokes.set(strokeId, strokeData);
    
    // 最初の点を描画（エラーが発生してもストロークIDは返す）
    try {
      this.drawSinglePoint(point, tool, color);
    } catch (error) {
      console.warn('Single point drawing failed, but stroke started:', error);
    }

    return strokeId;
  }

  addPoint(strokeId: StrokeId, point: Point): void {
    if (!this.drawingContext) {
      throw new Error('Worker context not initialized');
    }

    const strokeData = this.activeStrokes.get(strokeId);
    if (!strokeData) {
      throw new Error(`Stroke ${strokeId} not found`);
    }

    // 前回の点を取得
    const lastIndex = strokeData.points.length - 2;
    if (lastIndex >= 0) {
      const lastX = strokeData.points[lastIndex];
      const lastY = strokeData.points[lastIndex + 1];
      
      // 前回の点から今回の点までの線分を描画
      this.drawLine(lastX, lastY, point.x, point.y, strokeData.tool, strokeData.color);
    }

    // 新しい点を追加
    strokeData.points.push(point.x, point.y);
  }

  endStroke(strokeId: StrokeId): void {
    if (!this.drawingContext) {
      throw new Error('Worker context not initialized');
    }

    const strokeData = this.activeStrokes.get(strokeId);
    if (strokeData && strokeData.points.length >= 2) {
      // ストローク全体を一度に描画（スムージング処理などが可能）
      const points = new Float32Array(strokeData.points);
      const color = new Float32Array([
        strokeData.color.r / 255,
        strokeData.color.g / 255,
        strokeData.color.b / 255,
        strokeData.color.a
      ]);
      
      try {
        console.log('Drawing complete stroke:', { pointCount: points.length / 2, color, size: strokeData.tool.size });
        this.drawingContext.draw_stroke(points, color, strokeData.tool.size);
      } catch (error) {
        console.error('Failed to draw complete stroke:', error);
        throw error;
      }
    }

    this.activeStrokes.delete(strokeId);
  }

  clear(): void {
    if (!this.drawingContext) {
      throw new Error('Worker context not initialized');
    }

    // 白でクリア
    this.drawingContext.clear(new Float32Array([1.0, 1.0, 1.0, 1.0]));
    this.activeStrokes.clear();
  }

  updateCanvas(): void {
    if (!this.drawingContext || !this.ctx || !this.imageData) {
      throw new Error('Worker context not initialized');
    }

    try {
      // SharedBufferが利用可能な場合
      if (this.sharedBuffer) {
        this.drawingContext.copy_to_shared_buffer();
        const buffer = new Uint8Array(this.sharedBuffer.buffer);
        this.imageData.data.set(buffer);
      } else {
        // SharedBufferが利用できない場合は、通常の方法でピクセルを取得
        const pixels = this.drawingContext.get_pixels();
        this.imageData.data.set(pixels);
      }

      // キャンバスに描画
      this.ctx.putImageData(this.imageData, 0, 0);
    } catch (error) {
      console.error('Failed to update canvas:', error);
      throw error;
    }
  }

  // 単一の点を描画（ストロークの開始点用）
  private drawSinglePoint(point: Point, tool: BrushTool, color: Color): void {
    if (!this.drawingContext) return;

    // 単一の点の場合、小さな円として描画するために近くの2点目を追加
    const offset = 0.01; // 非常に小さなオフセット
    const points = new Float32Array([point.x, point.y, point.x + offset, point.y + offset]);
    const colorArray = new Float32Array([
      color.r / 255,
      color.g / 255,
      color.b / 255,
      color.a
    ]);
    
    try {
      console.log('Drawing single point:', { 
        point, 
        pointsArray: Array.from(points),
        color: Array.from(colorArray), 
        size: tool.size 
      });
      this.drawingContext.draw_stroke(points, colorArray, tool.size);
    } catch (error) {
      console.error('Failed to draw single point:', error);
      console.error('DrawingContext state:', {
        exists: !!this.drawingContext,
        hasDrawStroke: this.drawingContext ? typeof this.drawingContext.draw_stroke : 'N/A'
      });
      // 単一点のエラーは無視して、次の点が来るのを待つ
      console.warn('Ignoring single point draw error, waiting for second point');
    }
  }

  // 2点間の線を描画
  private drawLine(x1: number, y1: number, x2: number, y2: number, tool: BrushTool, color: Color): void {
    if (!this.drawingContext) return;

    // 線分を複数の点で補間（スムーズな線のため）
    const distance = Math.sqrt((x2 - x1) ** 2 + (y2 - y1) ** 2);
    const steps = Math.max(2, Math.ceil(distance / 2)); // 2ピクセルごとに1点
    
    const points: number[] = [];
    for (let i = 0; i <= steps; i++) {
      const t = i / steps;
      points.push(
        x1 + (x2 - x1) * t,
        y1 + (y2 - y1) * t
      );
    }

    const pointsArray = new Float32Array(points);
    const colorArray = new Float32Array([
      color.r / 255,
      color.g / 255,
      color.b / 255,
      color.a
    ]);
    
    try {
      console.log('Drawing line segment:', { points: pointsArray.length / 2, color: colorArray, size: tool.size });
      this.drawingContext.draw_stroke(pointsArray, colorArray, tool.size);
    } catch (error) {
      console.error('Failed to draw line segment:', error);
      throw error;
    }
  }

  destroy(): void {
    if (this.drawingContext) {
      this.drawingContext.free();
    }
    if (this.sharedBuffer) {
      this.sharedBuffer.free();
    }
    this.drawingContext = null;
    this.sharedBuffer = null;
    this.ctx = null;
    this.imageData = null;
    this.activeStrokes.clear();
  }
}

// Workerインスタンス
const workerContext = new WorkerContext();

// メッセージハンドラ
self.addEventListener('message', async (event: MessageEvent<IncomingMessage>) => {
  const message = event.data;
  const response: WorkerResponse = {
    type: 'error',
    id: message.id,
  };

  try {
    switch (message.type) {
      case 'init': {
        await workerContext.initialize(
          message.canvas,
          message.wasmUrl,
          message.width,
          message.height
        );
        response.type = 'initialized';
        break;
      }

      case 'startStroke': {
        const strokeId = workerContext.startStroke(
          message.point,
          message.tool,
          message.color
        );
        response.type = 'strokeStarted';
        response.data = { strokeId };
        break;
      }

      case 'addPoint': {
        workerContext.addPoint(message.strokeId, message.point);
        response.type = 'pointAdded';
        break;
      }

      case 'endStroke': {
        workerContext.endStroke(message.strokeId);
        response.type = 'strokeEnded';
        break;
      }

      case 'clear': {
        workerContext.clear();
        response.type = 'cleared';
        break;
      }

      case 'updateCanvas': {
        workerContext.updateCanvas();
        response.type = 'canvasUpdated';
        break;
      }

      default:
        throw new Error(`Unknown message type: ${(message as any).type}`);
    }
  } catch (error) {
    response.type = 'error';
    response.error = error instanceof Error ? error.message : 'Unknown error';
  }

  // レスポンスを送信
  self.postMessage(response);
});

// Workerの終了処理
self.addEventListener('unload', () => {
  workerContext.destroy();
});

// SharedArrayBufferサポートの確認
if (typeof SharedArrayBuffer !== 'undefined') {
  console.log('SharedArrayBuffer is supported in this worker');
} else {
  console.warn('SharedArrayBuffer is not supported in this worker');
}

// TypeScript用のエクスポート（Workerでは実際には使用されない）
export type { WorkerMessage, WorkerResponse };