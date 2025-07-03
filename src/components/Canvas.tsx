import { useCallback } from "react";
import { useCanvas } from "../lib/useCanvas";
import { useAtom } from "jotai";
import { toolAtom, brushSettingsAtom, drawingEngineAtom } from "../store/atoms";
import { DrawingCanvas } from "./DrawingCanvas";

interface CanvasProps {
  width: number;
  height: number;
}


export function Canvas({ width, height }: CanvasProps) {
  const [drawingEngine] = useAtom(drawingEngineAtom);
  
  // Tauriエンジンを使用する場合は新しいDrawingCanvasを使用
  if (drawingEngine === 'tauri') {
    return <DrawingCanvas width={width} height={height} />;
  }
  
  // 新しい統一されたCanvas描画フックを使用
  const { canvasRef, isReady, isDrawing, startDrawing, draw, endDrawing, clear } = useCanvas({ width, height });

  const [currentTool] = useAtom(toolAtom);
  const [brushSettings] = useAtom(brushSettingsAtom);

  // ポインターイベントハンドラ
  const handlePointerDown = useCallback((e: React.PointerEvent<HTMLCanvasElement>) => {
    const rect = canvasRef.current?.getBoundingClientRect();
    if (!rect || !isReady) return;

    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const pressure = e.pressure || 1.0;

    startDrawing(x, y, pressure);
    canvasRef.current?.setPointerCapture(e.pointerId);
  }, [isReady, startDrawing]);

  const handlePointerMove = useCallback((e: React.PointerEvent<HTMLCanvasElement>) => {
    if (!isDrawing || !isReady) return;

    const rect = canvasRef.current?.getBoundingClientRect();
    if (!rect) return;

    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const pressure = e.pressure || 1.0;

    draw(x, y, pressure);
  }, [isDrawing, isReady, draw]);

  const handlePointerUp = useCallback((e: React.PointerEvent<HTMLCanvasElement>) => {
    endDrawing();
    canvasRef.current?.releasePointerCapture(e.pointerId);
  }, [endDrawing]);


  return (
    <div className="canvas-container canvas-grid relative">
      <div className="flex items-center justify-center h-full">
        {/* Canvas要素は常にレンダリング */}
        <canvas
          ref={canvasRef}
          width={width}
          height={height}
          className="border border-secondary-600 bg-white cursor-crosshair shadow-lg"
          onPointerDown={handlePointerDown}
          onPointerMove={handlePointerMove}
          onPointerUp={handlePointerUp}
          onPointerLeave={handlePointerUp}
          onPointerCancel={handlePointerUp}
          style={{ touchAction: 'none' }}
        />
      </div>
      
      {/* ローディングオーバーレイ */}
      {!isReady && (
        <div className="absolute inset-0 flex items-center justify-center bg-white bg-opacity-80 pointer-events-none">
          <div className="text-center">
            <div className="animate-spin w-8 h-8 border-4 border-blue-500 border-t-transparent rounded-full mx-auto mb-2"></div>
            <p className="text-gray-600">
              描画エンジンを初期化中...
            </p>
          </div>
        </div>
      )}
      
      {/* 描画エンジン状態インジケーター（開発用） */}
      {process.env.NODE_ENV === 'development' && isReady && (
        <div className="absolute top-2 right-2 text-xs text-gray-500 bg-white bg-opacity-75 p-1 rounded">
          {drawingEngine === 'wasmWorker' ? 'WASM Worker' : drawingEngine === 'wasm' ? 'WASM Direct' : 'Canvas 2D'}
        </div>
      )}
      
      {/* デバッグ情報（開発時のみ） */}
      <div className="absolute bottom-4 left-4 text-xs text-gray-500 bg-white bg-opacity-75 p-2 rounded">
        <div>Engine: {drawingEngine === 'wasmWorker' ? 'WASM Worker' : drawingEngine === 'wasm' ? 'WASM Direct' : 'Canvas 2D'}</div>
        <div>Tool: {currentTool}</div>
        <div>Size: {brushSettings.size}px</div>
        <div>Opacity: {Math.round(brushSettings.opacity * 100)}%</div>
        <div>Drawing: {isDrawing ? 'Yes' : 'No'}</div>
      </div>
      
      {/* クリアボタン（開発時のみ） */}
      <button
        onClick={clear}
        className="absolute top-4 right-4 px-3 py-1 bg-red-500 text-white rounded hover:bg-red-600 transition-colors"
      >
        Clear
      </button>
    </div>
  );
}