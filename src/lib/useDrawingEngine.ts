import { useState, useEffect, useCallback } from 'react';
import { initializeDrawingEngine, createDrawingLayer, removeLayer } from './tauri.ts';

export interface DrawingEngine {
  createLayer: (layerId: string, width: number, height: number) => Promise<void>;
  removeLayer: (layerId: string) => Promise<void>;
  renderToCanvas: (canvas: HTMLCanvasElement, layerId: string) => Promise<void>;
}

export function useDrawingEngine() {
  const [drawingEngine, setDrawingEngine] = useState<DrawingEngine | null>(null);
  const [isInitialized, setIsInitialized] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  // 描画エンジンの初期化
  useEffect(() => {
    let isMounted = true;

    const initEngine = async () => {
      try {
        setError(null);
        await initializeDrawingEngine();
        
        if (isMounted) {
          const engine: DrawingEngine = {
            createLayer: async (layerId: string, width: number, height: number) => {
              await createDrawingLayer(layerId, width, height);
            },
            removeLayer: async (layerId: string) => {
              await removeLayer(layerId);
            },
            renderToCanvas: async (canvas: HTMLCanvasElement, _layerId: string) => {
              // TODO: 実際の描画処理を実装
              const ctx = canvas.getContext('2d');
              if (ctx) {
                ctx.fillStyle = '#333';
                ctx.fillRect(0, 0, canvas.width, canvas.height);
              }
            }
          };
          
          setDrawingEngine(engine);
          setIsInitialized(true);
        }
      } catch (err) {
        if (isMounted) {
          const errorMessage = err instanceof Error ? err.message : String(err);
          setError(errorMessage);
          setIsInitialized(false);
        }
      }
    };

    initEngine();

    return () => {
      isMounted = false;
    };
  }, []);

  return {
    drawingEngine,
    isInitialized,
    error,
    clearError
  };
}