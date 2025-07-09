import React, { useEffect, useState } from "react";
import { useDrawingEngine } from "../lib/useDrawingEngine";
import { BrushSettings, BlendMode, BrushType } from "../lib/bindings";
import { useAtom } from "jotai";
import { activeToolAtom, brushSizeAtom, colorAtom } from "../store/atoms";

interface DrawingCanvasProps {
    width: number;
    height: number;
}

export const DrawingCanvas: React.FC<DrawingCanvasProps> = ({ width, height }) => {
    const {
        canvasRef,
        state,
        initEngine,
        setBrush,
        updateCanvas,
        handlePointerDown,
        handlePointerMove,
        handlePointerUp,
        compareTransferMethods,
        useBinary,
        setUseBinary,
    } = useDrawingEngine({
        useBinary: true,
        updateInterval: 33, // 30fps
        moveEventThrottle: 10, // 2回に1回処理
        distanceThreshold: 2, // 2ピクセル以上の移動で処理
        debugMode: false, // デバッグログは無効
    });

    const [activeTool] = useAtom(activeToolAtom);
    const [brushSize] = useAtom(brushSizeAtom);
    const [color] = useAtom(colorAtom);
    const [isInitializing, setIsInitializing] = useState(true);

    // 描画エンジンの初期化
    useEffect(() => {
        const init = async () => {
            // 既に初期化済みの場合はスキップ
            if (state.isInitialized) {
                setIsInitializing(false);
                return;
            }

            try {
                await initEngine(width, height);
                await updateCanvas(false); // 初回は全体更新
                setIsInitializing(false);
            } catch (error) {
                console.error("Failed to initialize drawing engine:", error);
                setIsInitializing(false);
            }
        };
        init();
    }, [initEngine, updateCanvas, width, height, state.isInitialized]);

    // ブラシ設定の更新
    useEffect(() => {
        if (!state.isInitialized) return;

        const brushType: BrushType = activeTool === "eraser" ? "eraser" : "pen";

        // カラーを[R, G, B, A]形式に変換
        const rgbaColor = hexToRgba(color);

        const brushSettings: BrushSettings = {
            size: brushSize,
            opacity: 1.0,
            color: rgbaColor,
            brushType,
            blendMode: "normal" as BlendMode,
        };

        setBrush(brushSettings).catch(console.error);
    }, [activeTool, brushSize, color, setBrush, state.isInitialized]);

    if (isInitializing) {
        return (
            <div className="flex items-center justify-center w-full h-full">
                <div className="text-gray-500">描画エンジンを初期化中...</div>
            </div>
        );
    }

    if (state.error) {
        return (
            <div className="flex items-center justify-center w-full h-full">
                <div className="text-red-500">エラー: {state.error}</div>
            </div>
        );
    }

    return (
        <div className="relative">
            <canvas
                ref={canvasRef}
                width={width}
                height={height}
                className="bg-white shadow-lg cursor-crosshair"
                onPointerDown={handlePointerDown}
                onPointerMove={handlePointerMove}
                onPointerUp={handlePointerUp}
                onPointerLeave={handlePointerUp}
                style={{ touchAction: "none" }}
            />

            {/* デバッグコントロール */}
            <div className="absolute top-2 right-2 bg-black bg-opacity-50 text-white p-2 rounded text-xs">
                <div>Binary Mode: {useBinary ? "ON" : "OFF"}</div>
                <div className="mt-1 text-gray-300">
                    <div>Update: 30fps (33ms)</div>
                    <div>Event: 1/{2}</div>
                    <div>Threshold: 2px</div>
                </div>
                <button
                    onClick={() => setUseBinary(!useBinary)}
                    className="mt-1 px-2 py-1 bg-blue-500 rounded hover:bg-blue-600"
                >
                    Toggle Binary
                </button>
                <button
                    onClick={() => compareTransferMethods()}
                    className="mt-1 ml-1 px-2 py-1 bg-green-500 rounded hover:bg-green-600"
                >
                    Compare
                </button>
            </div>
        </div>
    );
};

// ヘルパー関数: HEXカラーをRGBA配列に変換
function hexToRgba(hex: string): [number, number, number, number] {
    const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
    if (result) {
        return [parseInt(result[1], 16) / 255.0, parseInt(result[2], 16) / 255.0, parseInt(result[3], 16) / 255.0, 1.0];
    }
    return [0.0, 0.0, 0.0, 1.0]; // デフォルトは黒
}
