# アニメーション制作アプリ仕様書

## プロジェクト概要
プロのイラストレーター向けのアニメーション制作（作画）アプリケーション。高パフォーマンスを実現するため、描画エンジンはTauriのRustプロセスでwgpuを使用して実装し、UIはReactで開発する。

## 技術スタック
- **フロントエンド**: React + TypeScript
- **パッケージマネージャー**: Deno
- **スタイリング**: TailwindCSS v4 + @tailwindcss/vite
- **バックエンド**: Rust (Tauri)
- **フレームワーク**: Tauri
- **描画エンジン**: 
  - **メイン**: Tauri IPC + wgpu (Rustネイティブ実装)
  - **レガシー**: WebAssembly (WASM) + wgpu
  - **フォールバック**: HTML5 Canvas 2D API
- **状態管理**: jotai
- **型安全性**: tauri-specta (Rust-TypeScript自動型生成)
- **プラグイン実行環境**: QuickJS-emscripten（将来実装）

### 主要依存関係
**Rust (Cargo.toml) - 現在インストール済み**
- tauri 2.6.2 (アプリケーションフレームワーク)
- tauri-build 2.3.0 (ビルドツール)
- serde 1.0 (シリアライゼーション)
- serde_json 1.0 (JSON処理)
- image 0.25 (画像処理)
- tokio 1.40 (非同期ランタイム)
- log 0.4 (ログ出力)
- env_logger 0.11 (ログ管理)
- wgpu 0.20 (GPU描画エンジン)
- bytemuck 1.16 (メモリレイアウト)
- lz4 1.28 (高速データ圧縮)
- specta 2.0.0-rc.20 (型生成)
- tauri-specta 2.0.0-rc.20 (Tauri型安全性)

**WASM描画エンジン (src-wasm/Cargo.toml)**
- wgpu 0.20 (WebGPU/WebGL描画エンジン)
- wasm-bindgen 0.2 (WASM-JSバインディング)
- web-sys 0.3 (Web API)
- js-sys 0.3 (JavaScript API)
- bytemuck 1.16 (メモリレイアウト)
- serde 1.0 (シリアライゼーション)
- serde_json 1.0 (JSON処理)
- wasm-bindgen-futures 0.4 (非同期処理)

**将来追加予定**
- roxmltree (xdts形式対応)

**React/TypeScript (package.json)**
- @tailwindcss/vite 4.1.11 (最新Vite統合)
- jotai 2.12.5 (状態管理)
- @tauri-apps/api 2.6.0 (Tauri API)
- vitest 2.1.9 (テストランナー)
- @testing-library/react 16.1.0 (テストライブラリ)
- jsdom 26.0.3 (DOM環境)
- @heroicons/react 2.2.0 (UIアイコン)

## 基本仕様
- **プロジェクト単位**: 1プロジェクト = 1カット
- **最大キャンバスサイズ**: 4K (3840×2160)
- **レイヤー数上限**: 10枚/フレーム
- **フレームレート**: 最大60fps対応
- **ペン遅延**: 限りなく少ない状態を目指す

## 機能要件

### 優先度：高（MVP）

#### 1. 描画基本機能
- ペン/ブラシツール（筆圧対応）
- 消しゴムツール
- 塗りつぶしツール
- 選択ツール（矩形、投げ縄、自動選択）
- 選択範囲の変形（移動、拡大縮小、回転）

#### 2. レイヤー機能
- 最大10レイヤー/フレーム
- 基本ブレンドモード（通常、乗算、スクリーン、オーバーレイ）
- 不透明度調整
- 固定レイヤー（全フレーム共通）

#### 3. アニメーション機能
- xdts形式（東映デジタルタイムシート）の読み込み/書き出し
- タイムシート編集
- オニオンスキン（前後5フレーム、透明度調整可）
- フレーム間のコピー＆ペースト
- 中割り機能

#### 4. ファイル入出力
- プロジェクトファイル（独自形式）
- 連番PNG/JPEG出力
- PSD読み込み（基本的なレイヤー構造のみ）

### 優先度：中

#### 1. 描画拡張機能
- ベクター描画/編集機能
- カスタムブラシ作成機能
- 基本的な図形ツール（円、四角、直線）
- 手ぶれ補正（工数次第で後回し可）

#### 2. アニメーション拡張
- GIFアニメーション出力
- MP4出力（ffmpeg連携）
- プレビュー機能の高度化

#### 3. UI/UX改善
- ショートカットカスタマイズ
- ワークスペースレイアウト保存

### 優先度：低（将来実装）

#### 1. プロフェッショナル機能
- カラープロファイル対応
- 音声トラック同期
- マルチプラットフォーム対応（Windows/Mac/Linux）

#### 2. プラグインシステム
- TypeScriptベースのプラグイン
- セキュアな実行環境（QuickJS-emscripten）
- 独自パッケージ形式での配布
- セマンティックバージョニング採用

## メモリ

### 開発メモ
- コードベースの変更を行った際は最終ステップで必ずTypeScriptのビルドチェックを行ってください

### コマンド
- 型定義の生成コマンド: cargo run --bin generate-bindings --features specta

## アーキテクチャ

### 描画エンジン
現在の実装では、TauriのRustプロセスでwgpuを使用した描画エンジンが動作しています。

#### ディレクトリ構造
```
src-tauri/src/
├── drawing_engine/      # 描画エンジンモジュール
│   ├── engine.rs       # メインエンジン
│   ├── renderer.rs     # wgpuレンダラー
│   ├── canvas_state.rs # キャンバス状態管理
│   ├── layer.rs        # レイヤー管理
│   ├── commands.rs     # 描画コマンド定義
│   ├── buffer.rs       # バッファ管理
│   ├── stroke.rs       # ストローク処理
│   └── compositor.rs   # レイヤー合成
└── ipc/                # IPC通信
    ├── handlers.rs     # コマンドハンドラー
    ├── binary.rs       # バイナリデータ転送
    └── diff_handlers.rs # 差分更新処理
```

#### 主要機能
1. **ライン補間**: ブレゼンハムのアルゴリズムによる滑らかな線描画
2. **ブレンドモード**: Normal、Multiply、Screen、Overlay
3. **レイヤー合成**: 最大10レイヤーの効率的な合成
4. **差分更新**: 変更領域のみの転送による高速更新
5. **データ圧縮**: LZ4による転送データの圧縮

### フロントエンド統合
- **useDrawingEngine**: 描画エンジンとの通信を管理するReactフック
- **DrawingCanvas**: Tauri描画エンジンを使用するキャンバスコンポーネント
- **LayerPanelDrawingEngine**: レイヤー管理UI

### 描画エンジンの切り替え
`drawingEngineAtom`の値により、以下のエンジンを切り替え可能：
- `'tauri'`: Tauri IPC描画エンジン（デフォルト）
- `'wasmWorker'`: Web Worker WASM実装
- `'wasm'`: 直接WASM実装
- `'canvas2d'`: Canvas 2D API