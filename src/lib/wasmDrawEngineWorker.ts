import type { DrawRequest, DrawResponse } from '../types/drawEngine.ts';

// Worker内で動作する描画エンジン
let drawEngine: any = null;
let wasmModule: any = null;
let canvasBuffer: SharedArrayBuffer | null = null;
let canvasView: Uint8ClampedArray | null = null;
let width = 0;
let height = 0;

// Workerメッセージハンドラ
self.addEventListener('message', async (event: MessageEvent<DrawRequest>) => {
  const { type, data } = event.data;

  try {
    switch (type) {
      case 'init': {
        // SharedArrayBufferとサイズを受け取る
        width = data.width;
        height = data.height;
        
        // WASMモジュールのロード
        if (!wasmModule) {
          const wasmUrl = new URL('/wasm/kinegraph_wasm.js', self.location.origin).href;
          const wasmBinaryUrl = new URL('/wasm/kinegraph_wasm_bg.wasm', self.location.origin).href;
          
          console.log('Loading WASM module from:', wasmUrl);
          
          try {
            // 動的インポートを使用
            wasmModule = await import(wasmUrl);
            console.log('WASM module loaded:', Object.keys(wasmModule));
            
            // WASMバイナリを初期化
            if (wasmModule.default) {
              await wasmModule.default(wasmBinaryUrl);
              console.log('WASM binary initialized');
            }
          } catch (error) {
            console.error('Failed to load WASM module:', error);
            throw new Error(`Failed to load WASM module: ${error}`);
          }
        }
        
        // DrawEngineの初期化
        if (!wasmModule.DrawEngine) {
          throw new Error('DrawEngine not found in WASM module');
        }
        
        // DrawEngineはasyncコンストラクタを持つ可能性がある
        try {
          drawEngine = await wasmModule.DrawEngine.new(width, height);
          console.log('DrawEngine created successfully');
        } catch (error) {
          console.error('Failed to create DrawEngine:', error);
          throw new Error(`Failed to create DrawEngine: ${error}`);
        }
        
        // DrawEngineのSharedArrayBufferを取得
        const sharedBuffer = drawEngine.get_shared_buffer();
        if (sharedBuffer) {
          canvasBuffer = sharedBuffer;
          canvasView = new Uint8ClampedArray(sharedBuffer);
          console.log('SharedArrayBuffer obtained from DrawEngine');
        } else {
          console.warn('SharedArrayBuffer not available from DrawEngine');
        }
        
        // 初期化完了を通知
        const response: DrawResponse = {
          type: 'initialized',
          data: { 
            success: true,
            sharedBuffer: canvasBuffer || undefined
          }
        };
        self.postMessage(response);
        break;
      }

      case 'startStroke': {
        if (!drawEngine) {
          throw new Error('DrawEngine not initialized');
        }
        
        const { point, tool, color, strokeId } = data;
        drawEngine.begin_stroke(
          point.x,
          point.y,
          point.pressure || 1.0,
          color.r,
          color.g,
          color.b,
          color.a * tool.opacity,
          0, // brush_type: 0 for normal brush
          tool.size
        );
        
        // レンダリング実行
        renderToSharedBuffer();
        
        const response: DrawResponse = {
          type: 'strokeStarted',
          data: { strokeId: strokeId }
        };
        self.postMessage(response);
        break;
      }

      case 'addPoint': {
        if (!drawEngine) {
          throw new Error('DrawEngine not initialized');
        }
        
        const { point } = data;
        drawEngine.add_point(
          point.x,
          point.y,
          point.pressure || 1.0
        );
        
        // レンダリング実行
        renderToSharedBuffer();
        
        const response: DrawResponse = {
          type: 'pointAdded',
          data: { success: true }
        };
        self.postMessage(response);
        break;
      }

      case 'endStroke': {
        if (!drawEngine) {
          throw new Error('DrawEngine not initialized');
        }
        
        // strokeIdは使用しない（DrawEngineが内部で管理）
        drawEngine.end_stroke();
        
        // レンダリング実行
        renderToSharedBuffer();
        
        const response: DrawResponse = {
          type: 'strokeEnded',
          data: { success: true }
        };
        self.postMessage(response);
        break;
      }

      case 'clear': {
        if (!drawEngine) {
          throw new Error('DrawEngine not initialized');
        }
        
        drawEngine.clear();
        
        // レンダリング実行
        renderToSharedBuffer();
        
        const response: DrawResponse = {
          type: 'cleared',
          data: { success: true }
        };
        self.postMessage(response);
        break;
      }

      case 'resize': {
        if (!drawEngine) {
          throw new Error('DrawEngine not initialized');
        }
        
        width = data.width;
        height = data.height;
        drawEngine.resize(width, height);
        
        // DrawEngineの新しいSharedArrayBufferを取得
        const newBuffer = drawEngine.get_shared_buffer();
        if (newBuffer) {
          canvasBuffer = newBuffer;
          canvasView = new Uint8ClampedArray(newBuffer);
        }
        
        // レンダリング実行
        renderToSharedBuffer();
        
        const response: DrawResponse = {
          type: 'resized',
          data: { success: true }
        };
        self.postMessage(response);
        break;
      }

      case 'destroy': {
        if (drawEngine) {
          drawEngine.free();
          drawEngine = null;
        }
        canvasBuffer = null;
        canvasView = null;
        
        const response: DrawResponse = {
          type: 'destroyed',
          data: { success: true }
        };
        self.postMessage(response);
        break;
      }
    }
  } catch (error) {
    // エラーレスポンスを送信
    const response: DrawResponse = {
      type: 'error',
      data: {
        message: error instanceof Error ? error.message : 'Unknown error',
        originalType: event.data.type
      }
    };
    self.postMessage(response);
  }
});

// SharedArrayBufferにレンダリング
function renderToSharedBuffer() {
  if (!drawEngine || !canvasView) {
    return;
  }
  
  // SharedArrayBufferに直接レンダリング
  drawEngine.render();
  
  // メインスレッドに更新通知
  const response: DrawResponse = {
    type: 'rendered',
    data: { timestamp: performance.now() }
  };
  self.postMessage(response);
}

// TypeScript用のexport（Workerでは実際には使われない）
export {};