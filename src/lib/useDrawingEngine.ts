import { useEffect, useRef, useCallback } from 'react';
import { drawingEngine, DrawingUpdate } from './drawingEngine';

/**
 * 描画エンジンとキャンバスを連携するカスタムフック
 */
export function useDrawingEngine(
  canvasRef: React.RefObject<HTMLCanvasElement>,
  layerId: string | null
) {
  const activeStrokeId = useRef<string | null>(null);

  // 描画更新ハンドラー
  const handleDrawingUpdate = useCallback((update: DrawingUpdate) => {
    if (!canvasRef.current) return;
    
    // 部分更新をキャンバスに適用
    drawingEngine.applyPartialUpdate(canvasRef.current, update);
  }, [canvasRef]);

  // リスナーの登録/解除
  useEffect(() => {
    if (!layerId) return;

    drawingEngine.addUpdateListener(layerId, handleDrawingUpdate);

    return () => {
      drawingEngine.removeUpdateListener(layerId, handleDrawingUpdate);
    };
  }, [layerId, handleDrawingUpdate]);

  // リアルタイムストロークを開始
  const beginStroke = useCallback(async (
    color: [number, number, number, number],
    brushSize: number,
    tool: string
  ) => {
    if (!layerId) {
      console.error('レイヤーIDが設定されていません');
      return;
    }

    try {
      const strokeId = await drawingEngine.beginRealtimeStroke(
        layerId,
        color,
        brushSize,
        tool
      );
      activeStrokeId.current = strokeId;
      console.debug(`ストローク開始: ${strokeId}`);
    } catch (error) {
      console.error('ストローク開始エラー:', error);
    }
  }, [layerId]);

  // ストロークに点を追加
  const addStrokePoint = useCallback(async (x: number, y: number, pressure: number) => {
    if (!activeStrokeId.current) return;

    try {
      await drawingEngine.addRealtimeStrokePoint(activeStrokeId.current, {
        x,
        y,
        pressure
      });
    } catch (error) {
      console.error('ストローク点追加エラー:', error);
    }
  }, []);

  // ストロークを完了
  const completeStroke = useCallback(async () => {
    if (!activeStrokeId.current) return;

    try {
      await drawingEngine.completeRealtimeStroke(activeStrokeId.current);
      console.debug(`ストローク完了: ${activeStrokeId.current}`);
      activeStrokeId.current = null;
    } catch (error) {
      console.error('ストローク完了エラー:', error);
    }
  }, []);

  return {
    beginStroke,
    addStrokePoint,
    completeStroke,
    isDrawing: () => activeStrokeId.current !== null
  };
}