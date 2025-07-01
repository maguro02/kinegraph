import { useRef, useEffect, useCallback, useState } from "react";
import { HybridDrawingEngine } from "./hybridDrawingEngine";
import { Canvas2DRenderer, type IRenderer } from "./renderer/canvasRenderer";
import type { DrawCommand, DrawingStateInfo, Point } from "./hybridCommands";

interface UseHybridDrawingOptions {
  width: number;
  height: number;
}

interface DrawingState {
  isDrawing: boolean;
  currentStroke: Point[];
  drawingState: DrawingStateInfo | null;
  commandQueue: DrawCommand[];
}

export function useHybridDrawing({ width, height }: UseHybridDrawingOptions) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const rendererRef = useRef<IRenderer | null>(null);
  const [state, setState] = useState<DrawingState>({
    isDrawing: false,
    currentStroke: [],
    drawingState: null,
    commandQueue: [],
  });

  // レンダラーの初期化
  useEffect(() => {
    const initRenderer = async () => {
      console.log("[useHybridDrawing] 初期化開始");
      
      if (!canvasRef.current) {
        console.log("[useHybridDrawing] Canvas要素がまだ準備されていません");
        return;
      }

      try {
        console.log("[useHybridDrawing] レンダラー作成中...");
        const renderer = new Canvas2DRenderer();
        await renderer.init(canvasRef.current);
        rendererRef.current = renderer;
        console.log("[useHybridDrawing] レンダラー初期化完了");

        // 初期状態を取得
        console.log("[useHybridDrawing] 描画状態を取得中...");
        const drawingState = await HybridDrawingEngine.getDrawingState();
        console.log("[useHybridDrawing] 描画状態取得完了:", drawingState);
        
        setState((prev) => ({ ...prev, drawingState }));
      } catch (error) {
        console.error("[useHybridDrawing] 初期化エラー:", error);
      }
    };

    initRenderer();

    return () => {
      if (rendererRef.current) {
        rendererRef.current.destroy();
        rendererRef.current = null;
      }
    };
  }, []);

  // コマンドキューの処理
  useEffect(() => {
    if (state.commandQueue.length > 0 && rendererRef.current) {
      rendererRef.current.executeCommands(state.commandQueue);
      setState((prev) => ({ ...prev, commandQueue: [] }));
    }
  }, [state.commandQueue]);

  // 描画開始
  const startDrawing = useCallback(
    (e: React.PointerEvent<HTMLCanvasElement>) => {
      const rect = canvasRef.current?.getBoundingClientRect();
      if (!rect) return;

      const point: Point = {
        x: e.clientX - rect.left,
        y: e.clientY - rect.top,
      };

      setState((prev) => ({
        ...prev,
        isDrawing: true,
        currentStroke: [point],
      }));
    },
    []
  );

  // 描画中
  const draw = useCallback(
    (e: React.PointerEvent<HTMLCanvasElement>) => {
      if (!state.isDrawing) return;

      const rect = canvasRef.current?.getBoundingClientRect();
      if (!rect) return;

      const point: Point = {
        x: e.clientX - rect.left,
        y: e.clientY - rect.top,
      };

      setState((prev) => ({
        ...prev,
        currentStroke: [...prev.currentStroke, point],
      }));

      // プレビュー描画（オプション）
      // ここで一時的な描画を行うことも可能
    },
    [state.isDrawing]
  );

  // 描画終了
  const endDrawing = useCallback(async () => {
    if (!state.isDrawing || state.currentStroke.length < 2) {
      setState((prev) => ({
        ...prev,
        isDrawing: false,
        currentStroke: [],
      }));
      return;
    }

    // アクティブレイヤーがない場合は、最初のレイヤーを使用
    const layerId =
      state.drawingState?.active_layer_id ||
      state.drawingState?.layers[0]?.id ||
      "default";

    try {
      // バックエンドに描画コマンドを送信
      const commands = await HybridDrawingEngine.drawStroke(
        state.currentStroke,
        state.drawingState?.current_color || "#000000",
        state.drawingState?.current_brush_size || 2.0,
        layerId
      );

      // コマンドをキューに追加
      setState((prev) => ({
        ...prev,
        isDrawing: false,
        currentStroke: [],
        commandQueue: [...prev.commandQueue, ...commands],
      }));
    } catch (error) {
      console.error("Failed to draw stroke:", error);
      setState((prev) => ({
        ...prev,
        isDrawing: false,
        currentStroke: [],
      }));
    }
  }, [state.isDrawing, state.currentStroke, state.drawingState]);

  // レイヤー作成
  const createLayer = useCallback(
    async (name: string) => {
      try {
        const commands = await HybridDrawingEngine.createLayer(name);
        setState((prev) => ({
          ...prev,
          commandQueue: [...prev.commandQueue, ...commands],
        }));

        // 状態を更新
        const drawingState = await HybridDrawingEngine.getDrawingState();
        setState((prev) => ({ ...prev, drawingState }));
      } catch (error) {
        console.error("Failed to create layer:", error);
      }
    },
    []
  );

  // ツール変更
  const changeTool = useCallback(async (toolId: string) => {
    try {
      await HybridDrawingEngine.changeTool(toolId);

      // 状態を更新
      const drawingState = await HybridDrawingEngine.getDrawingState();
      setState((prev) => ({ ...prev, drawingState }));
    } catch (error) {
      console.error("Failed to change tool:", error);
    }
  }, []);

  // Undo/Redo
  const undo = useCallback(async () => {
    try {
      const commands = await HybridDrawingEngine.undo();
      setState((prev) => ({
        ...prev,
        commandQueue: [...prev.commandQueue, ...commands],
      }));
    } catch (error) {
      console.error("Failed to undo:", error);
    }
  }, []);

  const redo = useCallback(async () => {
    try {
      const commands = await HybridDrawingEngine.redo();
      setState((prev) => ({
        ...prev,
        commandQueue: [...prev.commandQueue, ...commands],
      }));
    } catch (error) {
      console.error("Failed to redo:", error);
    }
  }, []);

  return {
    canvasRef,
    startDrawing,
    draw,
    endDrawing,
    createLayer,
    changeTool,
    undo,
    redo,
    drawingState: state.drawingState,
    width,
    height,
  };
}