import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { createProject, getSystemInfo, initializeDebugLogging } from '../tauri';

/**
 * create_project APIのエラー再現テスト
 * 
 * このテストスイートは以下のエラーシナリオを再現します：
 * 1. 正常なcreate_project呼び出し
 * 2. 無効なパラメータでの呼び出し
 * 3. Tauriバックエンドが未初期化の状態での呼び出し
 * 4. 複数回の連続呼び出し
 * 5. 非同期処理中の割り込み
 */

describe('Tauri API Error Reproduction Tests', () => {
  const consoleSpies = {
    info: vi.spyOn(console, 'info').mockImplementation(() => {}),
    error: vi.spyOn(console, 'error').mockImplementation(() => {}),
    debug: vi.spyOn(console, 'debug').mockImplementation(() => {}),
    warn: vi.spyOn(console, 'warn').mockImplementation(() => {}),
  };

  beforeEach(() => {
    // 各テスト前にモックをリセット
    globalThis.mockTauriInvoke.mockClear();
    Object.values(consoleSpies).forEach(spy => spy.mockClear());
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('createProject - 正常ケース', () => {
    it('有効なパラメータで正常にプロジェクトを作成', async () => {
      // モックの戻り値を設定
      const mockProject = {
        id: 'test-project-id',
        name: 'Test Project',
        width: 1920,
        height: 1080,
        frameRate: 24,
        frames: [],
        createdAt: new Date().toISOString()
      };

      globalThis.mockTauriInvoke.mockResolvedValueOnce(mockProject);

      // テスト実行
      const result = await createProject('Test Project', 1920, 1080, 24);

      // 検証
      expect(globalThis.mockTauriInvoke).toHaveBeenCalledWith('create_project', {
        args: {
          name: 'Test Project',
          width: 1920,
          height: 1080,
          frameRate: 24
        }
      });

      expect(result).toEqual(mockProject);
      expect(consoleSpies.info).toHaveBeenCalledWith(
        expect.stringContaining('createProject 関数呼び出し開始'),
        ''
      );
      expect(consoleSpies.info).toHaveBeenCalledWith(
        expect.stringContaining('create_project コマンド正常完了'),
        ''
      );
    });
  });

  describe('createProject - 無効なパラメータエラー', () => {
    it('空文字のプロジェクト名でエラー', async () => {
      const errorMessage = 'Project name cannot be empty';
      globalThis.mockTauriInvoke.mockRejectedValueOnce(new Error(errorMessage));

      await expect(createProject('', 1920, 1080, 24)).rejects.toThrow(errorMessage);

      expect(consoleSpies.error).toHaveBeenCalledWith(
        expect.stringContaining('create_project コマンドでエラーが発生'),
        expect.any(Error)
      );
    });

    it('負の値の幅でエラー', async () => {
      const errorMessage = 'Width must be positive';
      globalThis.mockTauriInvoke.mockRejectedValueOnce(new Error(errorMessage));

      await expect(createProject('Test', -100, 1080, 24)).rejects.toThrow(errorMessage);
    });

    it('ゼロの高さでエラー', async () => {
      const errorMessage = 'Height must be positive';
      globalThis.mockTauriInvoke.mockRejectedValueOnce(new Error(errorMessage));

      await expect(createProject('Test', 1920, 0, 24)).rejects.toThrow(errorMessage);
    });

    it('無効なフレームレートでエラー', async () => {
      const errorMessage = 'Frame rate must be between 1 and 120';
      globalThis.mockTauriInvoke.mockRejectedValueOnce(new Error(errorMessage));

      await expect(createProject('Test', 1920, 1080, 0)).rejects.toThrow(errorMessage);
    });

    it('極端に大きなキャンバスサイズでエラー', async () => {
      const errorMessage = 'Canvas size exceeds maximum allowed dimensions';
      globalThis.mockTauriInvoke.mockRejectedValueOnce(new Error(errorMessage));

      await expect(createProject('Test', 50000, 50000, 24)).rejects.toThrow(errorMessage);
    });
  });

  describe('createProject - バックエンド未初期化エラー', () => {
    it('Tauriバックエンドが未初期化の状態でエラー', async () => {
      const errorMessage = 'Tauri backend not initialized';
      globalThis.mockTauriInvoke.mockRejectedValueOnce(new Error(errorMessage));

      await expect(createProject('Test', 1920, 1080, 24)).rejects.toThrow(errorMessage);

      expect(consoleSpies.error).toHaveBeenCalledWith(
        expect.stringContaining('create_project コマンドでエラーが発生'),
        expect.any(Error)
      );
    });

    it('IPCチャンネルエラー', async () => {
      const errorMessage = 'IPC channel disconnected';
      globalThis.mockTauriInvoke.mockRejectedValueOnce(new Error(errorMessage));

      await expect(createProject('Test', 1920, 1080, 24)).rejects.toThrow(errorMessage);
    });

    it('コマンド未登録エラー', async () => {
      const errorMessage = 'Command create_project is not registered';
      globalThis.mockTauriInvoke.mockRejectedValueOnce(new Error(errorMessage));

      await expect(createProject('Test', 1920, 1080, 24)).rejects.toThrow(errorMessage);
    });
  });

  describe('createProject - 複数回連続呼び出し', () => {
    it('同時に複数のプロジェクト作成要求', async () => {
      // 最初の呼び出しは成功
      const mockProject1 = { id: 'project-1', name: 'Project 1' };
      const mockProject2 = { id: 'project-2', name: 'Project 2' };

      globalThis.mockTauriInvoke
        .mockResolvedValueOnce(mockProject1)
        .mockResolvedValueOnce(mockProject2);

      // 同時に複数の呼び出し
      const promises = [
        createProject('Project 1', 1920, 1080, 24),
        createProject('Project 2', 1920, 1080, 30)
      ];

      const results = await Promise.all(promises);

      expect(results).toHaveLength(2);
      expect(results[0]).toEqual(mockProject1);
      expect(results[1]).toEqual(mockProject2);
      expect(globalThis.mockTauriInvoke).toHaveBeenCalledTimes(2);
    });

    it('連続呼び出し中の2回目でエラー', async () => {
      const mockProject = { id: 'project-1', name: 'Project 1' };
      const errorMessage = 'Resource busy';

      globalThis.mockTauriInvoke
        .mockResolvedValueOnce(mockProject)
        .mockRejectedValueOnce(new Error(errorMessage));

      // 1回目は成功
      const result1 = await createProject('Project 1', 1920, 1080, 24);
      expect(result1).toEqual(mockProject);

      // 2回目はエラー
      await expect(createProject('Project 2', 1920, 1080, 30)).rejects.toThrow(errorMessage);
    });
  });

  describe('createProject - 非同期処理の割り込み', () => {
    it('タイムアウトエラー', async () => {
      const errorMessage = 'Operation timed out';
      globalThis.mockTauriInvoke.mockRejectedValueOnce(new Error(errorMessage));

      await expect(createProject('Test', 1920, 1080, 24)).rejects.toThrow(errorMessage);
    });

    it('プロセス中断エラー', async () => {
      const errorMessage = 'Process was interrupted';
      globalThis.mockTauriInvoke.mockRejectedValueOnce(new Error(errorMessage));

      await expect(createProject('Test', 1920, 1080, 24)).rejects.toThrow(errorMessage);
    });

    it('メモリ不足エラー', async () => {
      const errorMessage = 'Out of memory';
      globalThis.mockTauriInvoke.mockRejectedValueOnce(new Error(errorMessage));

      await expect(createProject('Test', 4096, 4096, 60)).rejects.toThrow(errorMessage);
    });
  });

  describe('createProject - エラー形式のテスト', () => {
    it('Error オブジェクトの詳細ログ', async () => {
      const error = new Error('Test error message');
      error.stack = 'Error stack trace';
      globalThis.mockTauriInvoke.mockRejectedValueOnce(error);

      await expect(createProject('Test', 1920, 1080, 24)).rejects.toThrow('Test error message');

      expect(consoleSpies.error).toHaveBeenCalledWith(
        expect.stringContaining('エラーメッセージ'),
        'Test error message'
      );
      expect(consoleSpies.error).toHaveBeenCalledWith(
        expect.stringContaining('エラースタック'),
        'Error stack trace'
      );
    });

    it('文字列エラーのログ', async () => {
      const errorMessage = 'String error message';
      globalThis.mockTauriInvoke.mockRejectedValueOnce(errorMessage);

      await expect(createProject('Test', 1920, 1080, 24)).rejects.toThrow(errorMessage);

      expect(consoleSpies.error).toHaveBeenCalledWith(
        expect.stringContaining('エラー文字列'),
        errorMessage
      );
    });

    it('未知のエラー形式のログ', async () => {
      const errorObject = { code: 500, message: 'Unknown error' };
      globalThis.mockTauriInvoke.mockRejectedValueOnce(errorObject);

      await expect(createProject('Test', 1920, 1080, 24)).rejects.toThrow();

      expect(consoleSpies.error).toHaveBeenCalledWith(
        expect.stringContaining('未知のエラー形式'),
        JSON.stringify(errorObject)
      );
    });
  });

  describe('getSystemInfo - エラーテスト', () => {
    it('システム情報取得の正常ケース', async () => {
      const mockSystemInfo = 'System info data';
      globalThis.mockTauriInvoke.mockResolvedValueOnce(mockSystemInfo);

      const result = await getSystemInfo();

      expect(result).toBe(mockSystemInfo);
      expect(globalThis.mockTauriInvoke).toHaveBeenCalledWith('get_system_info');
    });

    it('システム情報取得エラー', async () => {
      const errorMessage = 'Failed to get system info';
      globalThis.mockTauriInvoke.mockRejectedValueOnce(new Error(errorMessage));

      await expect(getSystemInfo()).rejects.toThrow(errorMessage);
    });
  });

  describe('initializeDebugLogging - 初期化テスト', () => {
    it('デバッグログ初期化の正常ケース', async () => {
      const mockSystemInfo = 'Debug system info';
      globalThis.mockTauriInvoke.mockResolvedValueOnce(mockSystemInfo);

      await initializeDebugLogging();

      expect(consoleSpies.info).toHaveBeenCalledWith(
        expect.stringContaining('デバッグログ初期化開始'),
        ''
      );
      expect(consoleSpies.info).toHaveBeenCalledWith(
        expect.stringContaining('システム初期化確認完了'),
        mockSystemInfo
      );
    });

    it('デバッグログ初期化エラー', async () => {
      const errorMessage = 'Debug initialization failed';
      globalThis.mockTauriInvoke.mockRejectedValueOnce(new Error(errorMessage));

      await initializeDebugLogging();

      expect(consoleSpies.error).toHaveBeenCalledWith(
        expect.stringContaining('システム初期化確認でエラー'),
        expect.any(Error)
      );
    });
  });

  describe('パフォーマンステスト', () => {
    it('大量の並列リクエスト', async () => {
      const numRequests = 10;
      const mockProject = { id: 'bulk-project', name: 'Bulk Project' };

      // 全てのリクエストを成功させる
      for (let i = 0; i < numRequests; i++) {
        globalThis.mockTauriInvoke.mockResolvedValueOnce({
          ...mockProject,
          id: `bulk-project-${i}`
        });
      }

      const promises = Array.from({ length: numRequests }, (_, i) =>
        createProject(`Bulk Project ${i}`, 1920, 1080, 24)
      );

      const results = await Promise.all(promises);

      expect(results).toHaveLength(numRequests);
      expect(globalThis.mockTauriInvoke).toHaveBeenCalledTimes(numRequests);
    });
  });
});