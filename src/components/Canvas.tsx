import { useEffect } from "react";
import { useHybridDrawing } from "../lib/useHybridDrawing";
import { useAtom } from "jotai";
import { selectedLayerAtom } from "../store/atoms";

interface CanvasProps {
  width: number;
  height: number;
}

export function Canvas({ width, height }: CanvasProps) {
  const {
    canvasRef,
    startDrawing,
    draw,
    endDrawing,
    drawingState,
  } = useHybridDrawing({ width, height });

  const [selectedLayerId, setSelectedLayerId] = useAtom(selectedLayerAtom);

  // 初期レイヤーの作成（Rust側で作成済みなので不要）
  useEffect(() => {
    if (drawingState) {
      console.log("[Canvas] 描画状態を取得:", drawingState);
    }
  }, [drawingState]);

  // アクティブレイヤーの設定
  useEffect(() => {
    if (drawingState && drawingState.layers.length > 0) {
      const firstLayerId = drawingState.layers[0].id;
      if (!selectedLayerId) {
        setSelectedLayerId(firstLayerId);
      }
    }
  }, [drawingState, selectedLayerId, setSelectedLayerId]);

  // 初期化中の表示
  if (!drawingState) {
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

  // レイヤーがない場合の表示
  if (drawingState.layers.length === 0) {
    return (
      <div className="canvas-container canvas-grid">
        <div className="flex items-center justify-center h-full">
          <div className="text-center text-gray-500">
            <p>レイヤーを作成しています...</p>
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
          onPointerDown={startDrawing}
          onPointerMove={draw}
          onPointerUp={endDrawing}
          onPointerLeave={endDrawing}
          onPointerCancel={endDrawing}
        />
      </div>
      
      {/* デバッグ情報（開発時のみ） */}
      <div className="absolute bottom-4 left-4 text-xs text-gray-500 bg-white bg-opacity-75 p-2 rounded">
        <div>Tool: {drawingState.current_tool}</div>
        <div>Color: {drawingState.current_color}</div>
        <div>Size: {drawingState.current_brush_size}px</div>
        <div>Active Layer: {drawingState.active_layer_id || "None"}</div>
        <div>Layers: {drawingState.layers.length}</div>
      </div>
    </div>
  );
}