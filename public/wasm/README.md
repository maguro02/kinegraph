# Kinegraph WebGPU Renderer

## 実装内容

このWebAssemblyモジュールは、KinegraphアプリケーションのためのWebGPUベースの高性能描画エンジンを提供します。

### 主な機能

1. **WebGPUレンダラー** (`WebGPURenderer`)
   - WebGPUデバイスとキューの管理
   - サーフェス設定とリサイズ対応

2. **描画コンテキスト** (`DrawingContext`)
   - オフスクリーンレンダリング対応
   - ストローク描画機能
   - クリア機能
   - ピクセルデータの読み取り
   - SharedArrayBufferサポート

3. **シェーダー実装**
   - 頂点シェーダー: ピクセル座標からNDCへの変換
   - フラグメントシェーダー: 単色塗りつぶし

4. **頂点データ管理**
   - ストローク幅を考慮した頂点生成
   - 効率的なバッファ管理

### 実装の特徴

- **高性能**: WebGPUを使用したGPUアクセラレーション
- **メモリ効率**: SharedArrayBufferを使用した高速データ転送
- **柔軟性**: 様々な描画操作に対応可能な設計

### ビルド方法

```bash
# 通常のビルド
wasm-pack build --target web

# 開発用ビルド（デバッグ情報付き）
wasm-pack build --dev --target web
```

### 使用例

```javascript
import init, { DrawingContext, SharedBuffer, check_webgpu_support } from './pkg/kinegraph_wasm.js';

async function main() {
    // WebGPUサポートチェック
    if (!check_webgpu_support()) {
        console.error('WebGPU is not supported');
        return;
    }

    // 初期化
    await init();

    // 描画コンテキスト作成
    const ctx = await DrawingContext.new(800, 600);

    // 背景クリア
    ctx.clear([1.0, 1.0, 1.0, 1.0]);

    // ストローク描画
    const points = new Float32Array([100, 100, 200, 200, 300, 150]);
    const color = [1.0, 0.0, 0.0, 1.0]; // 赤
    ctx.draw_stroke(points, color, 5.0);

    // ピクセルデータ取得
    const pixels = ctx.get_pixels();
}
```

### 今後の拡張予定

1. **高度なシェーダー**
   - テクスチャサポート
   - グラデーション
   - アンチエイリアシング

2. **パフォーマンス最適化**
   - インスタンシング
   - バッチレンダリング
   - カリング

3. **追加機能**
   - ブレンドモード
   - マスキング
   - エフェクト

### 注意事項

- WebGPUはまだ実験的な技術です
- ブラウザサポートを確認してください
- フォールバック実装も検討してください