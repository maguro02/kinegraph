import { useRef, useEffect, MouseEvent, useCallback, useState } from 'react';
import { useAtom } from 'jotai';
import { toolAtom, brushSizeAtom, colorAtom, selectedLayerAtom, currentFrameAtom } from '../store/atoms';
import { invoke } from '@tauri-apps/api/core';

interface CanvasProps {
  width: number;
  height: number;
}

interface StrokePoint {
  x: number;
  y: number; 
  pressure: number;
}

// 色文字列をRGBA配列に変換するヘルパー関数
function hexToRgba(hex: string, alpha: number = 1.0): [number, number, number, number] {
  const r = parseInt(hex.slice(1, 3), 16) / 255;
  const g = parseInt(hex.slice(3, 5), 16) / 255;
  const b = parseInt(hex.slice(5, 7), 16) / 255;
  return [r, g, b, alpha];
}

// カスタムフック: Rustドローエンジンの管理
function useDrawingEngine() {
  const [isInitialized, setIsInitialized] = useState(false);
  const [isInitializing, setIsInitializing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const initialize = useCallback(async () => {
    if (isInitialized || isInitializing) return;
    
    setIsInitializing(true);
    setError(null);
    
    try {
      console.log('[Canvas] 描画エンジン初期化開始');
      await invoke('initialize_drawing_engine');
      setIsInitialized(true);
      console.log('[Canvas] 描画エンジン初期化完了');
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      setError(errorMsg);
      console.error('[Canvas] 描画エンジン初期化エラー:', errorMsg);
    } finally {
      setIsInitializing(false);
    }
  }, [isInitialized, isInitializing]);

  return { isInitialized, isInitializing, error, initialize };
}

// カスタムフック: キャンバス描画機能
function useCanvasDrawing(layerId: string | null, width: number, height: number) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const imageDataRef = useRef<ImageData | null>(null);
  const [currentTool] = useAtom(toolAtom);
  const [brushSize] = useAtom(brushSizeAtom);
  const [color] = useAtom(colorAtom);
  
  const isDrawing = useRef(false);
  const strokePoints = useRef<StrokePoint[]>([]);
  const lastPoint = useRef<{ x: number; y: number } | null>(null);

  // レイヤー画像データをキャンバスに描画
  const updateCanvasFromLayer = useCallback(async () => {
    if (!layerId || !canvasRef.current) return;
    
    try {
      console.log('[Canvas] レイヤー画像データ取得中:', layerId);
      const imageData = await invoke<number[]>('get_layer_image_data', { layerId });
      
      const canvas = canvasRef.current;
      const ctx = canvas.getContext('2d');
      if (!ctx) return;

      // RGBA バイトデータをImageDataに変換
      const imgData = ctx.createImageData(width, height);
      const data = new Uint8Array(imageData);
      imgData.data.set(data);
      
      // キャンバスにデータを描画
      ctx.putImageData(imgData, 0, 0);
      imageDataRef.current = imgData;
      
      console.log('[Canvas] キャンバス更新完了');
    } catch (err) {
      console.error('[Canvas] レイヤー画像データ取得エラー:', err);
    }
  }, [layerId, width, height]);

  // ストローク描画をRustエンジンに送信
  const drawStrokeToEngine = useCallback(async (points: StrokePoint[]) => {
    if (!layerId || points.length === 0) return;
    
    try {
      const rgba = hexToRgba(color);
      console.log('[Canvas] ストローク描画:', { layerId, pointsCount: points.length, color: rgba });
      
      await invoke('draw_stroke_on_layer', {
        layerId,
        points,
        color: rgba,
      });
      
      // 描画後にキャンバスを更新
      await updateCanvasFromLayer();
    } catch (err) {
      console.error('[Canvas] ストローク描画エラー:', err);
    }
  }, [layerId, color, updateCanvasFromLayer]);

  // 線描画をRustエンジンに送信
  const drawLineToEngine = useCallback(async (x1: number, y1: number, x2: number, y2: number) => {
    if (!layerId) return;
    
    try {
      const rgba = hexToRgba(color);
      console.log('[Canvas] 線描画:', { layerId, from: [x1, y1], to: [x2, y2], color: rgba });
      
      await invoke('draw_line_on_layer', {
        layerId,
        x1, y1, x2, y2,
        color: rgba,
        width: brushSize,
      });
      
      // 描画後にキャンバスを更新
      await updateCanvasFromLayer();
    } catch (err) {
      console.error('[Canvas] 線描画エラー:', err);
    }
  }, [layerId, color, brushSize, updateCanvasFromLayer]);

  // マウスイベントハンドラー
  const startDrawing = useCallback((e: MouseEvent<HTMLCanvasElement>) => {
    if (!layerId || !canvasRef.current) return;
    
    const rect = canvasRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    
    isDrawing.current = true;
    lastPoint.current = { x, y };
    strokePoints.current = [{ x, y, pressure: 1.0 }]; // デフォルト筆圧
    
    console.log('[Canvas] 描画開始:', { x, y, tool: currentTool });
  }, [layerId, currentTool]);

  const draw = useCallback((e: MouseEvent<HTMLCanvasElement>) => {
    if (!isDrawing.current || !lastPoint.current || !layerId || !canvasRef.current) return;
    
    const rect = canvasRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    
    // ストローク点を追加
    strokePoints.current.push({ x, y, pressure: 1.0 });
    
    // リアルタイムプレビュー（HTML5 Canvas使用）
    const ctx = canvasRef.current.getContext('2d');
    if (ctx) {
      ctx.globalCompositeOperation = currentTool === 'eraser' ? 'destination-out' : 'source-over';
      ctx.strokeStyle = currentTool === 'eraser' ? 'rgba(0,0,0,1)' : color;
      ctx.lineWidth = brushSize;
      ctx.lineCap = 'round';
      ctx.lineJoin = 'round';
      
      ctx.beginPath();
      ctx.moveTo(lastPoint.current.x, lastPoint.current.y);
      ctx.lineTo(x, y);
      ctx.stroke();
    }
    
    lastPoint.current = { x, y };
  }, [layerId, currentTool, color, brushSize]);

  const stopDrawing = useCallback(async () => {
    if (!isDrawing.current || strokePoints.current.length === 0) {
      isDrawing.current = false;
      lastPoint.current = null;
      strokePoints.current = [];
      return;
    }
    
    console.log('[Canvas] 描画終了、Rustエンジンに送信中');
    
    // Rustエンジンにストロークを送信
    await drawStrokeToEngine(strokePoints.current);
    
    // 状態をリセット
    isDrawing.current = false;
    lastPoint.current = null;
    strokePoints.current = [];
  }, [drawStrokeToEngine]);

  return {
    canvasRef,
    startDrawing,
    draw,
    stopDrawing,
    updateCanvasFromLayer,
    drawLineToEngine,
  };
}

export function Canvas({ width, height }: CanvasProps) {
  const { isInitialized, isInitializing, error, initialize } = useDrawingEngine();
  const [selectedLayerId] = useAtom(selectedLayerAtom);
  const [currentFrame] = useAtom(currentFrameAtom);
  
  // レイヤーIDを決定（選択されたレイヤーまたはデフォルト）
  const activeLayerId = selectedLayerId || (currentFrame?.layers?.[0]?.id) || null;
  
  const {
    canvasRef,
    startDrawing,
    draw,
    stopDrawing,
    updateCanvasFromLayer,
  } = useCanvasDrawing(activeLayerId, width, height);

  // エンジン初期化とレイヤー作成
  useEffect(() => {
    initialize();
  }, [initialize]);

  // アクティブレイヤーが変更されたときの処理
  useEffect(() => {
    const setupLayer = async () => {
      if (!isInitialized || !activeLayerId) return;
      
      try {
        console.log('[Canvas] レイヤー作成中:', activeLayerId);
        await invoke('create_drawing_layer', {
          layerId: activeLayerId,
          width,
          height,
        });
        
        // レイヤーデータでキャンバスを更新
        await updateCanvasFromLayer();
      } catch (err) {
        // レイヤーが既に存在する場合はそのまま更新
        console.log('[Canvas] レイヤー既存、データ更新のみ実行');
        await updateCanvasFromLayer();
      }
    };
    
    setupLayer();
  }, [isInitialized, activeLayerId, width, height, updateCanvasFromLayer]);

  // エラー状態の表示
  if (error) {
    return (
      <div className="canvas-container canvas-grid">
        <div className="flex items-center justify-center h-full">
          <div className="text-red-500 text-center p-4">
            <p className="text-lg font-semibold mb-2">描画エンジンエラー</p>
            <p className="text-sm">{error}</p>
            <button 
              onClick={initialize}
              className="mt-4 px-4 py-2 bg-red-500 text-white rounded hover:bg-red-600"
            >
              再試行
            </button>
          </div>
        </div>
      </div>
    );
  }

  // 初期化中の表示
  if (isInitializing) {
    return (
      <div className="canvas-container canvas-grid">
        <div className="flex items-center justify-center h-full">
          <div className="text-center">
            <div className="animate-spin w-8 h-8 border-4 border-blue-500 border-t-transparent rounded-full mx-auto mb-2"></div>
            <p className="text-gray-600">描画エンジンを初期化中...</p>
          </div>
        </div>
      </div>
    );
  }

  // 初期化前の表示
  if (!isInitialized) {
    return (
      <div className="canvas-container canvas-grid">
        <div className="flex items-center justify-center h-full">
          <div className="text-center text-gray-500">
            <p>描画エンジンを準備中...</p>
          </div>
        </div>
      </div>
    );
  }

  // レイヤーが選択されていない場合
  if (!activeLayerId) {
    return (
      <div className="canvas-container canvas-grid">
        <div className="flex items-center justify-center h-full">
          <div className="text-center text-gray-500">
            <p>レイヤーを選択してください</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="canvas-container canvas-grid">
      <div className="flex items-center justify-center h-full">
        <canvas
          ref={canvasRef}
          width={width}
          height={height}
          className="border border-secondary-600 bg-white cursor-crosshair shadow-lg"
          onMouseDown={startDrawing}
          onMouseMove={draw}
          onMouseUp={stopDrawing}
          onMouseLeave={stopDrawing}
        />
      </div>
    </div>
  );
}