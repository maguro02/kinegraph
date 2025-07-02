/**
 * Canvas描画管理フック
 * 描画エンジンの種類に応じて適切な実装を選択
 */

import { useRef, useCallback, useEffect, useState } from 'react';
import { useAtom } from 'jotai';
import { toolAtom, brushSettingsAtom, drawingEngineAtom } from '../store/atoms';
import type { DrawingContext, Point, StrokeId, Color, BrushTool } from '../types/drawing';
import { createCanvas2DContext } from './canvas2dContext';
import { createWasmDirectContext } from './wasmDirect';
import { createWasmCanvasContext } from './wasmCanvas';

interface UseCanvasOptions {
    width: number;
    height: number;
}

interface UseCanvasReturn {
    canvasRef: React.RefObject<HTMLCanvasElement | null>;
    isReady: boolean;
    isDrawing: boolean;
    startDrawing: (x: number, y: number, pressure: number) => void;
    draw: (x: number, y: number, pressure: number) => void;
    endDrawing: () => void;
    clear: () => void;
}

export function useCanvas({ width, height }: UseCanvasOptions): UseCanvasReturn {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const drawingContextRef = useRef<DrawingContext | null>(null);
    const [isReady, setIsReady] = useState(false);
    const [isDrawing, setIsDrawing] = useState(false);
    const currentStrokeIdRef = useRef<StrokeId | null>(null);
    
    const [currentTool] = useAtom(toolAtom);
    const [brushSettings] = useAtom(brushSettingsAtom);
    const [drawingEngine] = useAtom(drawingEngineAtom);

    // 描画エンジンの初期化
    useEffect(() => {
        const initEngine = async () => {
            if (!canvasRef.current) return;
            
            // 既存のコンテキストをクリーンアップ
            if (drawingContextRef.current) {
                drawingContextRef.current.destroy();
                drawingContextRef.current = null;
            }
            
            setIsReady(false);
            
            try {
                console.log(`[useCanvas] ${drawingEngine}描画エンジンを初期化中...`);
                
                let context: DrawingContext;
                
                switch (drawingEngine) {
                    case 'canvas2d':
                        context = createCanvas2DContext(canvasRef.current, width, height);
                        break;
                    case 'wasm':
                        context = await createWasmDirectContext(canvasRef.current, width, height);
                        break;
                    case 'wasmWorker':
                        context = await createWasmCanvasContext(canvasRef.current, width, height);
                        break;
                    default:
                        // フォールバック
                        context = createCanvas2DContext(canvasRef.current, width, height);
                }
                
                await context.initialize();
                drawingContextRef.current = context;
                
                console.log(`[useCanvas] ${drawingEngine}描画エンジン初期化完了`);
                setIsReady(true);
            } catch (error) {
                console.error('[useCanvas] 描画エンジン初期化エラー:', error);
                
                // エラー時はCanvas2Dにフォールバック
                if (canvasRef.current && drawingEngine !== 'canvas2d') {
                    try {
                        console.log('[useCanvas] Canvas2Dにフォールバック...');
                        const fallbackContext = createCanvas2DContext(canvasRef.current, width, height);
                        await fallbackContext.initialize();
                        drawingContextRef.current = fallbackContext;
                        setIsReady(true);
                    } catch (fallbackError) {
                        console.error('[useCanvas] フォールバックも失敗:', fallbackError);
                    }
                }
            }
        };

        initEngine();
        
        // クリーンアップ
        return () => {
            if (drawingContextRef.current) {
                drawingContextRef.current.destroy();
                drawingContextRef.current = null;
            }
        };
    }, [width, height, drawingEngine]);

    // 色をColor型に変換
    const hexToColor = useCallback((hex: string): Color => {
        const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
        return result ? {
            r: parseInt(result[1], 16),
            g: parseInt(result[2], 16),
            b: parseInt(result[3], 16),
            a: 1.0
        } : { r: 0, g: 0, b: 0, a: 1.0 };
    }, []);

    // ストローク開始
    const startDrawing = useCallback(async (x: number, y: number, pressure: number) => {
        if (!isReady || !drawingContextRef.current) return;

        setIsDrawing(true);

        // 現在のツールに応じて処理
        if (currentTool === 'pen' || currentTool === 'eraser') {
            try {
                const point: Point = { x, y, pressure };
                const tool: BrushTool = {
                    type: 'pen',
                    size: brushSettings.size,
                    opacity: brushSettings.opacity,
                    smoothing: 0.5,
                    pressure: true
                };
                const color = currentTool === 'eraser' 
                    ? { r: 255, g: 255, b: 255, a: 1.0 }
                    : hexToColor(brushSettings.color);
                
                const strokeId = await drawingContextRef.current.startStroke(point, tool, color);
                currentStrokeIdRef.current = strokeId;
            } catch (error) {
                console.error('[useCanvas] ストローク開始エラー:', error);
            }
        }
    }, [isReady, currentTool, brushSettings, hexToColor]);

    // ストローク描画
    const draw = useCallback(async (x: number, y: number, pressure: number) => {
        if (!isDrawing || !isReady || !drawingContextRef.current || !currentStrokeIdRef.current) return;

        try {
            const point: Point = { x, y, pressure };
            await drawingContextRef.current.addPoint(currentStrokeIdRef.current, point);
        } catch (error) {
            console.error('[useCanvas] ポイント追加エラー:', error);
        }
    }, [isDrawing, isReady]);

    // ストローク終了
    const endDrawing = useCallback(async () => {
        if (!isDrawing || !isReady || !drawingContextRef.current || !currentStrokeIdRef.current) return;

        setIsDrawing(false);

        try {
            await drawingContextRef.current.endStroke(currentStrokeIdRef.current);
            currentStrokeIdRef.current = null;
        } catch (error) {
            console.error('[useCanvas] ストローク終了エラー:', error);
        }
    }, [isDrawing, isReady]);

    // キャンバスをクリア
    const clear = useCallback(async () => {
        if (!drawingContextRef.current) return;
        
        try {
            await drawingContextRef.current.clear();
        } catch (error) {
            console.error('[useCanvas] クリアエラー:', error);
        }
    }, []);

    return {
        canvasRef,
        isReady,
        isDrawing,
        startDrawing,
        draw,
        endDrawing,
        clear
    };
}