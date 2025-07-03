import { useRef, useCallback, useEffect, useState } from 'react';
import { useAtom } from 'jotai';
import { commands, DrawEngineCommand, BrushSettings } from './bindings';
import { drawingEngineStateAtom } from '../store/atoms';

export function useDrawingEngine() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [state, setState] = useAtom(drawingEngineStateAtom);

  // 描画エンジンの初期化
  const initEngine = useCallback(async (width: number, height: number) => {
    try {
      const result = await commands.initCanvas(width, height);
      if (result.status === 'ok') {
        setState({
          canvasId: result.data,
          isInitialized: true,
          error: null,
        });
        return result.data;
      } else {
        setState(prev => ({
          ...prev,
          error: result.error,
        }));
        throw new Error(result.error);
      }
    } catch (error) {
      setState(prev => ({
        ...prev,
        error: error instanceof Error ? error.message : String(error),
      }));
      throw error;
    }
  }, []);

  // 描画コマンドの送信
  const sendCommand = useCallback(async (command: DrawEngineCommand) => {
    if (!state.canvasId || !state.isInitialized) {
      throw new Error('Drawing engine not initialized');
    }

    const result = await commands.drawCommand(command);
    if (result.status === 'error') {
      throw new Error(result.error);
    }
  }, [state.canvasId, state.isInitialized]);

  // レンダリング結果の取得と表示
  const updateCanvas = useCallback(async (useDiff: boolean = true) => {
    if (!state.canvasId || !canvasRef.current) {
      return;
    }

    const ctx = canvasRef.current.getContext('2d');
    if (!ctx) return;

    try {
      if (useDiff) {
        // 差分更新を使用
        const result = await commands.getDiffRenderResult(state.canvasId);
        if (result.status === 'ok' && result.data.dirtyRegion) {
          const region = result.data.dirtyRegion;
          const imageData = new ImageData(
            new Uint8ClampedArray(region.imageData.data),
            region.imageData.width,
            region.imageData.height
          );
          ctx.putImageData(imageData, region.x, region.y);
        }
      } else {
        // 全体更新を使用
        const result = await commands.getRenderResult(state.canvasId);
        if (result.status === 'ok') {
          const renderData = result.data;
          const imageData = new ImageData(
            new Uint8ClampedArray(renderData.imageData.data),
            renderData.imageData.width,
            renderData.imageData.height
          );
          ctx.putImageData(imageData, 0, 0);
        }
      }
    } catch (error) {
      console.error('Failed to update canvas:', error);
    }
  }, [state.canvasId]);

  // キャンバスのリサイズ
  const resizeCanvas = useCallback(async (width: number, height: number) => {
    if (!state.canvasId) {
      throw new Error('Canvas not initialized');
    }

    const result = await commands.resizeCanvas(state.canvasId, width, height);
    if (result.status === 'error') {
      throw new Error(result.error);
    }
  }, [state.canvasId]);

  // ブラシ設定のヘルパー関数
  const setBrush = useCallback(async (settings: BrushSettings) => {
    await sendCommand({ setBrush: settings });
  }, [sendCommand]);

  // ストローク操作のヘルパー関数
  const beginStroke = useCallback(async (x: number, y: number, pressure: number = 1.0) => {
    await sendCommand({ beginStroke: { x, y, pressure } });
  }, [sendCommand]);

  const continueStroke = useCallback(async (x: number, y: number, pressure: number = 1.0) => {
    await sendCommand({ continueStroke: { x, y, pressure } });
  }, [sendCommand]);

  const endStroke = useCallback(async () => {
    await sendCommand('endStroke');
  }, [sendCommand]);

  // レイヤー操作のヘルパー関数
  const createLayer = useCallback(async () => {
    await sendCommand('createLayer');
  }, [sendCommand]);

  const deleteLayer = useCallback(async (index: number) => {
    await sendCommand({ deleteLayer: index });
  }, [sendCommand]);

  const setActiveLayer = useCallback(async (index: number) => {
    await sendCommand({ setActiveLayer: index });
  }, [sendCommand]);

  // キャンバスクリア
  const clearCanvas = useCallback(async () => {
    await sendCommand('clear');
  }, [sendCommand]);

  // ポインターイベントハンドラー
  const [isDrawing, setIsDrawing] = useState(false);

  const handlePointerDown = useCallback(async (e: React.PointerEvent<HTMLCanvasElement>) => {
    if (!canvasRef.current) return;
    
    const rect = canvasRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const pressure = e.pressure || 1.0;
    
    setIsDrawing(true);
    await beginStroke(x, y, pressure);
    await updateCanvas();
  }, [beginStroke, updateCanvas]);

  const handlePointerMove = useCallback(async (e: React.PointerEvent<HTMLCanvasElement>) => {
    if (!isDrawing || !canvasRef.current) return;
    
    const rect = canvasRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const pressure = e.pressure || 1.0;
    
    await continueStroke(x, y, pressure);
    await updateCanvas();
  }, [isDrawing, continueStroke, updateCanvas]);

  const handlePointerUp = useCallback(async () => {
    if (!isDrawing) return;
    
    setIsDrawing(false);
    await endStroke();
    await updateCanvas();
  }, [isDrawing, endStroke, updateCanvas]);

  // キャンバスがアンマウントされたときにキャッシュをクリアし、状態をリセット
  useEffect(() => {
    return () => {
      if (state.canvasId) {
        commands.clearRenderCache(state.canvasId).catch(console.error);
        // 状態をリセット
        setState({
          canvasId: null,
          isInitialized: false,
          error: null,
        });
      }
    };
  }, [state.canvasId, setState]);

  return {
    canvasRef,
    state,
    initEngine,
    sendCommand,
    updateCanvas,
    resizeCanvas,
    setBrush,
    beginStroke,
    continueStroke,
    endStroke,
    createLayer,
    deleteLayer,
    setActiveLayer,
    clearCanvas,
    // ポインターイベントハンドラー
    handlePointerDown,
    handlePointerMove,
    handlePointerUp,
  };
}