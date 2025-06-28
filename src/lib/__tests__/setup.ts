import '@testing-library/jest-dom';

// Tauri API のモック設定
const mockInvoke = vi.fn();

// グローバルなモック設定
Object.defineProperty(window, '__TAURI__', {
  value: {
    core: {
      invoke: mockInvoke
    }
  },
  configurable: true
});

// Tauri APIのモック
vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke
}));

// テスト前後でモックをリセット
beforeEach(() => {
  mockInvoke.mockClear();
});

afterEach(() => {
  vi.clearAllMocks();
});

// テスト用のヘルパー関数をグローバルに追加
declare global {
  var mockTauriInvoke: typeof mockInvoke;
}

globalThis.mockTauriInvoke = mockInvoke;