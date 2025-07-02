/// <reference lib="webworker" />

// テスト用Workerファイル

self.addEventListener('message', async (event) => {
  if (event.data.type === 'test') {
    try {
      // WASMモジュールのJSファイルをfetchで読み込み
      const jsUrl = new URL('/wasm/kinegraph_wasm.js', self.location.origin).href;
      const jsResponse = await fetch(jsUrl);
      
      if (!jsResponse.ok) {
        throw new Error(`Failed to fetch WASM JS in worker: ${jsResponse.status}`);
      }
      
      const jsText = await jsResponse.text();
      
      // モジュールが存在することを確認（実際の実行はしない）
      if (jsText.includes('export') && jsText.includes('init')) {
        self.postMessage({ success: true });
      } else {
        throw new Error('Invalid WASM module structure');
      }
      
    } catch (error) {
      self.postMessage({ 
        success: false, 
        error: error instanceof Error ? error.message : 'Unknown error' 
      });
    }
  }
});