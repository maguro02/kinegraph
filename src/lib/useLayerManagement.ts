import { useState, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

/**
 * レイヤー管理用のカスタムフック
 * LayerPanelなど、canvas要素を持たないコンポーネントで使用
 */
export function useLayerManagement() {
  const [isInitialized, setIsInitialized] = useState(false);
  const [isInitializing, setIsInitializing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 描画エンジンの初期化
  const initializeEngine = useCallback(async () => {
    if (isInitialized || isInitializing) return;
    
    setIsInitializing(true);
    setError(null);
    
    try {
      console.log('[LayerManagement] 描画エンジン初期化開始');
      await invoke('initialize_drawing_engine');
      setIsInitialized(true);
      console.log('[LayerManagement] 描画エンジン初期化完了');
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      setError(errorMsg);
      console.error('[LayerManagement] 描画エンジン初期化エラー:', errorMsg);
    } finally {
      setIsInitializing(false);
    }
  }, [isInitialized, isInitializing]);

  // エラーをクリア
  const clearError = useCallback(() => {
    setError(null);
  }, []);

  // コンポーネントマウント時に初期化
  useEffect(() => {
    initializeEngine();
  }, [initializeEngine]);

  return {
    isInitialized,
    isInitializing,
    error,
    clearError,
    initializeEngine
  };
}