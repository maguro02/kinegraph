// Test script to verify WASM memory management
import('./public/wasm/kinegraph_wasm.js').then(async (wasmModule) => {
  console.log('=== WASM Memory Management Test ===');
  
  try {
    // 初期化前の確認
    console.log('\n1. WASM exports before init:', Object.keys(wasmModule));
    console.log('   - DrawingContext:', typeof wasmModule.DrawingContext);
    console.log('   - check_webgpu_support:', typeof wasmModule.check_webgpu_support);
    
    // デフォルトエクスポートを実行（初期化）
    if (wasmModule.default) {
      console.log('\n2. Initializing WASM module...');
      await wasmModule.default();
      console.log('   - Initialization complete');
    }
    
    // 初期化後の再確認
    console.log('\n3. WASM exports after init:', Object.keys(wasmModule));
    
    // DrawingContextのインスタンス作成テスト
    if (wasmModule.DrawingContext) {
      console.log('\n4. Creating DrawingContext instance...');
      try {
        const context = await new wasmModule.DrawingContext(800, 600);
        console.log('   - DrawingContext created successfully');
        console.log('   - Context methods:', Object.getOwnPropertyNames(Object.getPrototypeOf(context)));
        
        // draw_strokeメソッドのテスト
        if (context.draw_stroke) {
          console.log('\n5. Testing draw_stroke method...');
          const testPoints = new Float32Array([100, 100, 200, 200]);
          const testColor = new Float32Array([1, 0, 0, 1]);
          const testWidth = 5;
          
          console.log('   - Input data:');
          console.log('     - Points:', testPoints);
          console.log('     - Color:', testColor);
          console.log('     - Width:', testWidth);
          
          try {
            context.draw_stroke(testPoints, testColor, testWidth);
            console.log('   - draw_stroke executed successfully');
          } catch (err) {
            console.error('   - draw_stroke error:', err);
            console.error('   - Error stack:', err.stack);
          }
        }
      } catch (err) {
        console.error('   - Failed to create DrawingContext:', err);
      }
    } else {
      console.error('   - DrawingContext not found in exports');
    }
    
    // WASMメモリの内部状態を確認
    console.log('\n6. Checking WASM internals...');
    if (wasmModule.__wbg_get_imports) {
      const imports = wasmModule.__wbg_get_imports();
      console.log('   - Import keys:', Object.keys(imports));
    }
    
    // __wbindgen_export_2の存在確認
    console.log('\n7. Memory allocation function check...');
    // これは実行時にwasmオブジェクトに設定される
    console.log('   - Note: __wbindgen_export_2 is set at runtime after initialization');
    
  } catch (error) {
    console.error('\nTest failed:', error);
    console.error('Stack:', error.stack);
  }
}).catch(err => {
  console.error('Failed to load WASM module:', err);
});