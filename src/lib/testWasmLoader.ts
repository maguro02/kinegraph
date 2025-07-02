// WASMモジュールの読み込みテスト用ユーティリティ

export async function testWasmLoading(): Promise<void> {
  console.log('Testing WASM module loading...');
  
  try {
    // 1. WASMバイナリファイルの存在確認
    const wasmUrl = new URL('/wasm/kinegraph_wasm_bg.wasm', window.location.origin).href;
    const wasmResponse = await fetch(wasmUrl);
    
    if (!wasmResponse.ok) {
      throw new Error(`Failed to fetch WASM file: ${wasmResponse.status} ${wasmResponse.statusText}`);
    }
    
    console.log('✓ WASM binary file found');
    
    // 2. WASMモジュールJSファイルの存在確認
    const jsUrl = new URL('/wasm/kinegraph_wasm.js', window.location.origin).href;
    const jsResponse = await fetch(jsUrl);
    
    if (!jsResponse.ok) {
      throw new Error(`Failed to fetch WASM JS file: ${jsResponse.status} ${jsResponse.statusText}`);
    }
    
    console.log('✓ WASM JS module found');
    
    // 3. WebGPUサポートの確認
    const hasWebGPU = 'gpu' in navigator;
    console.log(`✓ WebGPU support: ${hasWebGPU ? 'Yes' : 'No'}`);
    
    // 4. SharedArrayBufferサポートの確認
    const hasSharedArrayBuffer = typeof SharedArrayBuffer !== 'undefined';
    console.log(`✓ SharedArrayBuffer support: ${hasSharedArrayBuffer ? 'Yes' : 'No'}`);
    
    // 5. OffscreenCanvasサポートの確認
    const hasOffscreenCanvas = typeof OffscreenCanvas !== 'undefined';
    console.log(`✓ OffscreenCanvas support: ${hasOffscreenCanvas ? 'Yes' : 'No'}`);
    
    // 6. Worker内でのWASMモジュール読み込みテスト
    const testWorker = new Worker(
      new URL('./testWasmWorker.ts', import.meta.url),
      { type: 'module' }
    );
    
    const workerTestPromise = new Promise<void>((resolve, reject) => {
      testWorker.onmessage = (e) => {
        if (e.data.success) {
          console.log('✓ Worker WASM loading test passed');
          resolve();
        } else {
          reject(new Error(e.data.error || 'Worker test failed'));
        }
      };
      
      testWorker.onerror = (error) => {
        reject(new Error(`Worker error: ${error}`));
      };
    });
    
    testWorker.postMessage({ type: 'test' });
    
    await workerTestPromise;
    testWorker.terminate();
    
    console.log('\n✅ All WASM loading tests passed!');
    
  } catch (error) {
    console.error('❌ WASM loading test failed:', error);
    throw error;
  }
}

// ブラウザのコンソールから実行できるようにグローバルに公開
if (typeof window !== 'undefined') {
  (window as any).testWasmLoading = testWasmLoading;
}