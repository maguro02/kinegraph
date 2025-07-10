// Worker内で動作する描画エンジン（最適化版）
let drawEngine = null;
let wasmModule = null;
let canvasBuffer = null;
let canvasView = null;
let width = 0;
let height = 0;

// バッチ処理用
let pendingPoints = [];
let batchTimer = null;
const BATCH_INTERVAL = 8; // 8ms = 120fps相当
let lastRenderTime = 0;
const MIN_RENDER_INTERVAL = 16; // 60fps

// ダーティリージョン管理
let dirtyRegion = null;

// Workerメッセージハンドラ
self.addEventListener('message', async (event) => {
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
            wasmModule = await import(wasmUrl);
            console.log('WASM module loaded:', Object.keys(wasmModule));
            
            if (wasmModule.default) {
              await wasmModule.default(wasmBinaryUrl);
              console.log('WASM binary initialized');
            }
          } catch (error) {
            console.error('Failed to load WASM module:', error);
            throw new Error(`Failed to load WASM module: ${error}`);
          }
        }
        
        // DrawEngineの初期化（WebGPUチェックはスキップ）
        try {
          console.log('Using fallback DrawEngine (non-WebGPU)');
          drawEngine = new wasmModule.DrawEngine(width, height);
          console.log('Drawing engine created successfully');
          
          // エンジンメソッドの確認
          console.log('Drawing engine methods:', {
            begin_stroke: typeof drawEngine.begin_stroke,
            add_point: typeof drawEngine.add_point,
            end_stroke: typeof drawEngine.end_stroke,
            clear: typeof drawEngine.clear,
            render: typeof drawEngine.render
          });
        } catch (error) {
          console.error('Failed to create drawing engine:', error);
          throw new Error(`Failed to create drawing engine: ${error}`);
        }
        
        // SharedArrayBufferを取得
        let sharedBuffer = null;
        try {
          if (typeof drawEngine.get_shared_buffer === 'function') {
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
        const response = {
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
        
        // 色を0-255に変換
        const r = Math.round(color.r);
        const g = Math.round(color.g);
        const b = Math.round(color.b);
        const a = Math.round(color.a * tool.opacity * 255);
        
        try {
          const stroke_id = drawEngine.begin_stroke(
            point.x,
            point.y,
            point.pressure || 1.0,
            r, g, b, a,
            0, // brush_type: 0 for normal brush
            tool.size
          );
          
          // ダーティリージョンをリセット
          dirtyRegion = null;
        } catch (error) {
          console.error('begin_stroke error:', error);
          throw error;
        }
        
        const response = {
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
        
        // バッチ処理のためポイントを蓄積
        pendingPoints.push(data.point);
        
        // ダーティリージョンを更新
        updateDirtyRegion(data.point);
        
        // バッチタイマーがなければ開始
        if (!batchTimer) {
          batchTimer = setTimeout(processPendingPoints, BATCH_INTERVAL);
        }
        break;
      }

      case 'endStroke': {
        if (!drawEngine) {
          throw new Error('DrawEngine not initialized');
        }
        
        // 保留中のポイントを処理
        if (pendingPoints.length > 0) {
          clearTimeout(batchTimer);
          processPendingPoints();
        }
        
        drawEngine.end_stroke();
        
        // 最終レンダリング
        scheduleRender(true);
        
        const response = {
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
        dirtyRegion = null;
        
        // 即座にレンダリング
        renderToSharedBuffer();
        
        const response = {
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
          if (typeof drawEngine.get_shared_buffer === 'function') {
            newBuffer = drawEngine.get_shared_buffer();
          }
          
          if (newBuffer) {
            canvasBuffer = newBuffer;
            canvasView = new Uint8ClampedArray(newBuffer);
          }
        } catch (error) {
          console.warn('Failed to get new SharedArrayBuffer after resize:', error);
        }
        
        dirtyRegion = null;
        renderToSharedBuffer();
        
        const response = {
          type: 'resized',
          data: { success: true }
        };
        self.postMessage(response);
        break;
      }

      case 'destroy': {
        if (batchTimer) {
          clearTimeout(batchTimer);
        }
        
        if (drawEngine) {
          drawEngine.free();
          drawEngine = null;
        }
        canvasBuffer = null;
        canvasView = null;
        pendingPoints = [];
        dirtyRegion = null;
        
        const response = {
          type: 'destroyed',
          data: { success: true }
        };
        self.postMessage(response);
        break;
      }
    }
  } catch (error) {
    const response = {
      type: 'error',
      data: {
        message: error instanceof Error ? error.message : 'Unknown error',
        originalType: event.data.type
      }
    };
    self.postMessage(response);
  }
});

// バッチ処理でポイントを追加
function processPendingPoints() {
  batchTimer = null;
  
  if (!drawEngine || pendingPoints.length === 0) return;
  
  // すべてのポイントを一度に処理
  for (const point of pendingPoints) {
    drawEngine.add_point(
      point.x,
      point.y,
      point.pressure || 1.0
    );
  }
  
  pendingPoints = [];
  
  // レンダリングをスケジュール
  scheduleRender(false);
}

// ダーティリージョンを更新
function updateDirtyRegion(point) {
  const padding = 50; // ブラシサイズに応じて調整
  
  if (!dirtyRegion) {
    dirtyRegion = {
      x: Math.max(0, point.x - padding),
      y: Math.max(0, point.y - padding),
      width: padding * 2,
      height: padding * 2
    };
  } else {
    // 既存のリージョンを拡張
    const minX = Math.min(dirtyRegion.x, point.x - padding);
    const minY = Math.min(dirtyRegion.y, point.y - padding);
    const maxX = Math.max(dirtyRegion.x + dirtyRegion.width, point.x + padding);
    const maxY = Math.max(dirtyRegion.y + dirtyRegion.height, point.y + padding);
    
    dirtyRegion = {
      x: Math.max(0, minX),
      y: Math.max(0, minY),
      width: Math.min(width - minX, maxX - minX),
      height: Math.min(height - minY, maxY - minY)
    };
  }
}

// レンダリングをスケジュール
function scheduleRender(immediate) {
  const now = performance.now();
  const timeSinceLastRender = now - lastRenderTime;
  
  if (immediate || timeSinceLastRender >= MIN_RENDER_INTERVAL) {
    renderToSharedBuffer();
  }
}

// SharedArrayBufferにレンダリング
function renderToSharedBuffer() {
  if (!drawEngine || !canvasView) {
    console.warn('renderToSharedBuffer: drawEngine or canvasView is null');
    return;
  }
  
  lastRenderTime = performance.now();
  
  try {
    // ダーティリージョンを取得
    const dirtyRegions = drawEngine.get_and_clear_dirty_regions();
    console.log('DirtyRegions from WASM:', dirtyRegions);
    console.log('JS dirtyRegion:', dirtyRegion);
    
    if (dirtyRegions && dirtyRegions.length > 0) {
      // 部分レンダリング
      console.log('Performing partial rendering');
      drawEngine.render_dirty_regions();
      
      // メインスレッドに更新通知（ダーティリージョン情報付き）
      const response = {
        type: 'rendered',
        data: { 
          timestamp: lastRenderTime,
          dirtyRegion: dirtyRegions[0], // 最初のマージされた領域を使用
          isPartialUpdate: true
        }
      };
      self.postMessage(response);
    } else if (dirtyRegion) {
      // フルレンダリング（フォールバック）
      console.log('Performing full rendering (JS dirtyRegion exists)');
      drawEngine.render();
      
      const response = {
        type: 'rendered',
        data: { 
          timestamp: lastRenderTime,
          dirtyRegion: dirtyRegion,
          isPartialUpdate: false
        }
      };
      self.postMessage(response);
    } else {
      // ダーティリージョンが全くない場合もフルレンダリング
      console.log('No dirty regions, performing full rendering');
      drawEngine.render();
      
      const response = {
        type: 'rendered',
        data: { 
          timestamp: lastRenderTime,
          dirtyRegion: null,
          isPartialUpdate: false
        }
      };
      self.postMessage(response);
    }
  } catch (error) {
    console.error('Render error:', error);
    // エラー時はフルレンダリングにフォールバック
    drawEngine.render();
    
    const response = {
      type: 'rendered',
      data: { 
        timestamp: lastRenderTime,
        dirtyRegion: null,
        isPartialUpdate: false
      }
    };
    self.postMessage(response);
  }
  
  // ダーティリージョンをリセット
  dirtyRegion = null;
}