# create_project エラー再現テストスイート

## 概要
このテストスイートは、Kinegraphアプリケーションの`create_project` Tauri APIのエラー再現と検証を目的として作成されました。フロントエンド側でのエラーハンドリングとロギングの動作を包括的にテストします。

## ファイル構成

### テスト関連ファイル
```
src/lib/__tests__/
├── tauri.test.ts           # メインテストファイル（22個のテストケース）
├── setup.ts                # Vitestセットアップ（Tauriモック設定）
├── manual-test.html        # ブラウザベースの手動テストページ
├── error-debugging-guide.md # デバッグガイド
└── README.md               # このファイル
```

### 設定ファイル
- `package.json` - Vitestと関連依存関係を追加
- `vite.config.ts` - Vitest設定（jsdom環境、カバレッジ等）

## テストケース一覧

### 1. 正常ケース (1件)
- ✅ 有効なパラメータでのプロジェクト作成

### 2. 無効パラメータエラー (5件)
- ❌ 空文字のプロジェクト名
- ❌ 負の値の幅
- ❌ ゼロの高さ
- ❌ 無効なフレームレート
- ❌ 極端に大きなキャンバスサイズ

### 3. バックエンド未初期化エラー (3件)
- ⚠️ Tauriバックエンド未初期化
- ⚠️ IPCチャンネルエラー
- ⚠️ コマンド未登録エラー

### 4. 複数回連続呼び出し (2件)
- 🔄 同時複数プロジェクト作成
- 🔄 連続呼び出し中のエラー

### 5. 非同期処理の割り込み (3件)
- ⏱️ タイムアウトエラー
- 🛑 プロセス中断エラー
- 💾 メモリ不足エラー

### 6. エラー形式テスト (3件)
- 📄 Errorオブジェクトの詳細ログ
- 📄 文字列エラーのログ
- 📄 未知のエラー形式のログ

### 7. システム情報テスト (2件)
- 🔧 getSystemInfo 正常・エラーケース

### 8. 初期化テスト (2件)
- 🔧 initializeDebugLogging 正常・エラーケース

### 9. パフォーマンステスト (1件)
- ⚡ 大量並列リクエスト処理

## テスト実行方法

### 基本的なテスト実行
```bash
# すべてのテストを実行
deno run test

# テストを1回実行（CIモード）
deno run test:run

# テストUIでインタラクティブに実行
deno run test:ui
```

### カバレッジ付きテスト
```bash
# カバレッジレポート生成
deno run test --coverage

# HTMLカバレッジレポート生成
deno run test --coverage --reporter=html
```

### 特定のテストファイル実行
```bash
# Tauriテストのみ実行
deno run test src/lib/__tests__/tauri.test.ts

# 特定のテストケースのみ実行
deno run test -t "正常ケース"
```

### 監視モード
```bash
# ファイル変更時に自動再実行
deno run test --watch
```

## 手動テスト

### ブラウザベーステスト
1. Tauriアプリケーションを起動:
   ```bash
   deno run tauri dev
   ```

2. ブラウザで手動テストページを開く:
   ```
   file:///path/to/kinegraph/src/lib/__tests__/manual-test.html
   ```

3. 開発者ツール（F12）のコンソールタブを開く

4. 各種エラーシナリオボタンをクリックしてテスト

### 実際のTauri環境でのテスト
手動テストページはTauri環境の有無を自動検出し、以下のように動作します：
- **Tauri環境**: 実際のTauri APIを呼び出し
- **ブラウザ環境**: モック関数でエラーシナリオを再現

## デバッグ方法

### 1. コンソールログ確認
すべてのAPI呼び出しは詳細なログを出力します：
```
[2025-06-28T19:13:16.546Z] [KINEGRAPH-FRONTEND] createProject 関数呼び出し開始
[2025-06-28T19:13:16.546Z] [KINEGRAPH-FRONTEND] プロジェクトパラメータ {name: "Test", width: 1920, height: 1080, frameRate: 24}
[2025-06-28T19:13:16.547Z] [KINEGRAPH-FRONTEND] Tauri invoke create_project 実行中...
❌ [2025-06-28T19:13:16.548Z] [KINEGRAPH-FRONTEND] create_project コマンドでエラーが発生 Error: Project name cannot be empty
```

### 2. テスト内でのデバッグ
```bash
# 詳細なテスト出力
deno run test --reporter=verbose

# 特定のテストのみデバッグ
deno run test -t "空文字名でエラー" --reporter=verbose
```

### 3. Rustバックエンドのログ
```bash
# デバッグログ付きでTauri起動
RUST_LOG=debug deno run tauri dev
```

## モック設定

テスト環境では以下のモックが自動で設定されます：

### Tauri APIモック
```typescript
// setup.ts で自動設定
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke
}));
```

### グローバルアクセス
```typescript
// テスト内でモックを制御
globalThis.mockTauriInvoke.mockResolvedValueOnce(mockResult);
globalThis.mockTauriInvoke.mockRejectedValueOnce(new Error('Test error'));
```

## よくあるエラーと解決方法

### 1. テスト環境でのタイムアウト
```bash
# タイムアウト時間を延長
deno run test --testTimeout=10000
```

### 2. モックが動作しない
- `setup.ts` がVitestに正しく読み込まれているか確認
- `globalThis.mockTauriInvoke` が利用可能か確認

### 3. 依存関係エラー
```bash
# 依存関係を再インストール
deno install
```

## 今後の拡張予定

### 1. 追加テストケース
- [ ] ネットワーク切断シナリオ
- [ ] メモリリークテスト
- [ ] 大容量ファイル処理テスト

### 2. 統合テスト
- [ ] 実際のRustバックエンドとの結合テスト
- [ ] End-to-Endテスト

### 3. パフォーマンステスト
- [ ] レスポンス時間計測
- [ ] 同時接続数制限テスト
- [ ] リソース使用量監視

## 参考資料

- [Vitest公式ドキュメント](https://vitest.dev/)
- [Testing Library公式ドキュメント](https://testing-library.com/)
- [Tauri APIドキュメント](https://tauri.app/v1/api/)
- [error-debugging-guide.md](./error-debugging-guide.md) - 詳細なデバッグ手順