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
        
        // WebGPUDrawEngineまたはDrawEngineの初期化
        try {
          if (wasmModule.WebGPUDrawEngine) {
            console.log('Using WebGPU-accelerated drawing engine');
            // WebGPUDrawEngineは通常のコンストラクタ
            drawEngine = new wasmModule.WebGPUDrawEngine(width, height);
          } else if (wasmModule.DrawEngine) {
            console.log('Using fallback drawing engine');
            drawEngine = new wasmModule.DrawEngine(width, height);
          } else {
            throw new Error('Neither WebGPUDrawEngine nor DrawEngine found in WASM module');
          }
          console.log('Drawing engine created successfully');
        } catch (error) {
          console.error('Failed to create drawing engine:', error);
          throw new Error(`Failed to create drawing engine: ${error}`);
        }
        
        // SharedArrayBufferを取得
        let sharedBuffer = null;
        try {
          // WebGPUDrawEngineはreadonly propertyとしてshared_bufferを持つ
          if (drawEngine.shared_buffer !== undefined) {
            sharedBuffer = drawEngine.shared_buffer;
          } else if (typeof drawEngine.get_shared_buffer === 'function') {
            sharedBuffer = drawEngine.get_shared_buffer();
          }
          
          if (sharedBuffer) {
            canvasBuffer = sharedBuffer;
            canvasView = new Uint8ClampedArray(sharedBuffer);
            console.log('SharedArrayBuffer obtained from drawing engine');
          } else {
            console.warn('SharedArrayBuffer not available from drawing engine');
          }
        } catch (error) {
          console.warn('Failed to get SharedArrayBuffer:', error);
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
        
        // 新しいSharedArrayBufferを取得
        try {
          let newBuffer = null;
          if (drawEngine.shared_buffer !== undefined) {
            newBuffer = drawEngine.shared_buffer;
          } else if (typeof drawEngine.get_shared_buffer === 'function') {
            newBuffer = drawEngine.get_shared_buffer();
          }
          
          if (newBuffer) {
            canvasBuffer = newBuffer;
            canvasView = new Uint8ClampedArray(newBuffer);
          }
        } catch (error) {
          console.warn('Failed to get new SharedArrayBuffer after resize:', error);
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