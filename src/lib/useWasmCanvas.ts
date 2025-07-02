import { useRef, useState, useEffect, useCallback } from "react";
import { initWasm } from "./wasmLoader";
import type { Point, BrushTool, Color, DrawingContext } from "../types/drawing";

interface StrokeOptions {
    points: Point[];
    color: Color;
    size: number;
    opacity: number;
    pressure: boolean;
}

// ストローク情報を保持するための型
interface StrokeInfo {
    id: string;
    color: Color;
    tool: BrushTool;
    points: Point[];
}

export const useWasmCanvas = (width: number, height: number) => {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const [isWasmReady, setIsWasmReady] = useState(false);
    const [canvas2d, setCanvas2d] = useState<CanvasRenderingContext2D | null>(null);
    const engineRef = useRef<DrawingContext | null>(null);
    const [error, setError] = useState<string | null>(null);
    const currentStrokeIdRef = useRef<string | null>(null);
    const [isInitializing, setIsInitializing] = useState(true);

    // 統合された初期化プロセス
    useEffect(() => {
        if (!canvasRef.current) return;

        let mounted = true;

        const initialize = async () => {
            try {
                // Canvas要素の存在を確認
                const canvas = canvasRef.current;
                if (!canvas || !mounted) return;

                console.log("描画エンジンの初期化を開始...");
                setIsInitializing(true);

                // 最初にWASMエンジンの初期化を試みる
                try {
                    const wasmModule = await initWasm();

                    if (mounted && wasmModule) {
                        // WebGPUサポートチェック
                        const hasWebGPU = await wasmModule.check_webgpu_support();
                        console.log("WebGPU supported:", hasWebGPU);

                        // Canvasにidを設定
                        if (!canvas.id) {
                            canvas.id = `canvas-${Date.now()}`;
                        }

                        // DrawingContextの作成
                        console.log("Creating DrawingContext...");
                        const drawingContext = await new wasmModule.DrawingContext(width, height);
                        console.log("DrawingContext created:", drawingContext);
                        
                        // DrawingContextが正しく作成されたか確認
                        if (!drawingContext || typeof drawingContext.draw_stroke !== 'function') {
                            throw new Error('DrawingContext was not properly initialized');
                        }

                        // DrawingContext APIに適合させるためのラッパー
                        const activeStrokes = new Map<string, StrokeInfo>();
                        const wasmEngine: DrawingContext = {
                            async initialize(): Promise<void> {
                                // 初期化処理（既に初期化済み）
                                drawingContext.clear(new Float32Array([1, 1, 1, 1]));
                            },
                            async startStroke(point: Point, tool: BrushTool, color: Color): Promise<string> {
                                // ポイントの検証
                                if (
                                    !point ||
                                    typeof point.x !== "number" ||
                                    typeof point.y !== "number" ||
                                    isNaN(point.x) ||
                                    isNaN(point.y)
                                ) {
                                    console.error("Invalid point provided to startStroke:", point);
                                    throw new Error("Invalid point coordinates");
                                }

                                // ストロークの開始
                                const strokeId = Date.now().toString();

                                // ストローク情報を保存
                                const strokeInfo: StrokeInfo = {
                                    id: strokeId,
                                    color,
                                    tool,
                                    points: [point],
                                };
                                activeStrokes.set(strokeId, strokeInfo);

                                // Rust側は最低2点（4つの値）を要求するため、最初の点は描画をスキップ
                                // 2点目が追加されたときに描画を開始する
                                console.log("Stroke started with point:", point, "waiting for second point to draw");
                                currentStrokeIdRef.current = strokeId;

                                return strokeId;
                            },
                            async addPoint(strokeId: string, point: Point): Promise<void> {
                                // ポイントの検証
                                if (
                                    !point ||
                                    typeof point.x !== "number" ||
                                    typeof point.y !== "number" ||
                                    isNaN(point.x) ||
                                    isNaN(point.y)
                                ) {
                                    console.error("Invalid point provided to addPoint:", point);
                                    return;
                                }

                                // ストローク情報を取得
                                const strokeInfo = activeStrokes.get(strokeId);
                                if (!strokeInfo) {
                                    console.error(`Stroke ${strokeId} not found`);
                                    return;
                                }

                                // ポイントの追加
                                strokeInfo.points.push(point);

                                // 現在のストローク全体を再描画
                                if (strokeInfo.points.length === 2) {
                                    // 最初の線分を描画（2点目が追加された時）
                                    try {
                                        const points = new Float32Array([
                                            strokeInfo.points[0].x,
                                            strokeInfo.points[0].y,
                                            strokeInfo.points[1].x,
                                            strokeInfo.points[1].y,
                                        ]);
                                        const colorArray = new Float32Array([
                                            strokeInfo.color.r / 255,
                                            strokeInfo.color.g / 255,
                                            strokeInfo.color.b / 255,
                                            strokeInfo.color.a,
                                        ]);
                                        
                                        console.log("Drawing initial stroke segment:", {
                                            pointsLength: points.length,
                                            pointsData: Array.from(points),
                                            colorData: Array.from(colorArray),
                                            width: strokeInfo.tool.size
                                        });
                                        
                                        drawingContext.draw_stroke(points, colorArray, strokeInfo.tool.size);
                                    } catch (error) {
                                        console.error("Error drawing initial stroke:", error);
                                        throw error;
                                    }
                                } else if (strokeInfo.points.length > 2) {
                                    // 後続の点を追加（最後の2点のみ描画）
                                    const lastIndex = strokeInfo.points.length - 1;
                                    try {
                                        const points = new Float32Array([
                                            strokeInfo.points[lastIndex - 1].x,
                                            strokeInfo.points[lastIndex - 1].y,
                                            strokeInfo.points[lastIndex].x,
                                            strokeInfo.points[lastIndex].y,
                                        ]);
                                        const colorArray = new Float32Array([
                                            strokeInfo.color.r / 255,
                                            strokeInfo.color.g / 255,
                                            strokeInfo.color.b / 255,
                                            strokeInfo.color.a,
                                        ]);
                                        
                                        console.log("Drawing stroke continuation:", {
                                            pointsLength: points.length,
                                            pointsData: Array.from(points)
                                        });
                                        
                                        drawingContext.draw_stroke(points, colorArray, strokeInfo.tool.size);
                                    } catch (error) {
                                        console.error("Error drawing stroke continuation:", error);
                                        throw error;
                                    }
                                }
                            },
                            async endStroke(strokeId: string): Promise<void> {
                                // ストロークの終了とクリーンアップ
                                const strokeInfo = activeStrokes.get(strokeId);
                                if (strokeInfo) {
                                    console.log(`Ending stroke ${strokeId} with ${strokeInfo.points.length} points`);
                                }
                                activeStrokes.delete(strokeId);
                                if (currentStrokeIdRef.current === strokeId) {
                                    currentStrokeIdRef.current = null;
                                }
                            },
                            async clear(): Promise<void> {
                                drawingContext.clear(new Float32Array([1, 1, 1, 1]));
                            },
                            resize(width: number, height: number): void {
                                drawingContext.resize(width, height);
                            },
                            destroy(): void {
                                // アクティブなストロークをクリア
                                activeStrokes.clear();
                                // DrawingContextのクリーンアップ（必要に応じて）
                            },
                        };

                        engineRef.current = wasmEngine;
                        setIsWasmReady(true);
                        console.log("WASM描画エンジンの初期化完了");
                    }
                } catch (wasmError) {
                    console.error("WASM初期化エラー:", wasmError);
                    setError(wasmError instanceof Error ? wasmError.message : "WASM初期化に失敗しました");

                    // WASMが失敗した場合のみCanvas 2Dにフォールバック
                    if (mounted) {
                        const ctx = canvas.getContext("2d");
                        if (ctx) {
                            setCanvas2d(ctx);
                            // 基本的な初期設定
                            ctx.fillStyle = "#ffffff";
                            ctx.fillRect(0, 0, width, height);
                            console.log("Canvas 2Dモードにフォールバック");
                        }
                    }
                }
            } finally {
                if (mounted) {
                    setIsInitializing(false);
                }
            }
        };

        // 少し遅延させてcanvasが確実にDOMに追加されるのを待つ
        const timer = setTimeout(() => {
            initialize();
        }, 50);

        return () => {
            mounted = false;
            clearTimeout(timer);
            engineRef.current?.destroy();
        };
    }, [width, height]);

    // 描画関数
    const drawStroke = useCallback(
        async (strokeOptions: StrokeOptions) => {
            const { points, color, size, opacity, pressure } = strokeOptions;

            // 入力検証
            if (!points || points.length === 0) {
                console.warn("No points provided for stroke");
                return;
            }

            // 色の値を正規化
            const normalizedColor: Color = {
                r: Math.max(0, Math.min(255, color.r)),
                g: Math.max(0, Math.min(255, color.g)),
                b: Math.max(0, Math.min(255, color.b)),
                a: Math.max(0, Math.min(1, color.a)),
            };

            if (isWasmReady && engineRef.current) {
                // WASMモード
                const tool: BrushTool = {
                    type: "pen",
                    size: Math.max(0.1, size),
                    opacity: Math.max(0, Math.min(1, opacity)),
                    smoothing: 0.5,
                    pressure,
                };

                try {
                    const strokeId = await engineRef.current.startStroke(points[0], tool, normalizedColor);
                    currentStrokeIdRef.current = strokeId;

                    // バッチ処理でパフォーマンス向上
                    const batchSize = 5;
                    for (let i = 1; i < points.length; i += batchSize) {
                        const batch = points.slice(i, Math.min(i + batchSize, points.length));
                        for (const point of batch) {
                            await engineRef.current.addPoint(strokeId, point);
                        }
                    }

                    await engineRef.current.endStroke(strokeId);
                    currentStrokeIdRef.current = null;
                } catch (err) {
                    console.error("WASM描画エラー:", err);
                    setError(err instanceof Error ? err.message : "描画中にエラーが発生しました");

                    // エラー時のクリーンアップ
                    if (currentStrokeIdRef.current) {
                        try {
                            await engineRef.current.endStroke(currentStrokeIdRef.current);
                        } catch (cleanupErr) {
                            console.error("クリーンアップエラー:", cleanupErr);
                        }
                        currentStrokeIdRef.current = null;
                    }
                }
            } else if (canvas2d) {
                // Canvas2Dモード
                try {
                    canvas2d.save();
                    canvas2d.strokeStyle = `rgba(${normalizedColor.r}, ${normalizedColor.g}, ${normalizedColor.b}, ${
                        normalizedColor.a * opacity
                    })`;
                    canvas2d.lineWidth = Math.max(0.1, size);
                    canvas2d.lineCap = "round";
                    canvas2d.lineJoin = "round";

                    canvas2d.beginPath();
                    canvas2d.moveTo(points[0].x, points[0].y);

                    // Quadratic curves for smoother lines
                    if (points.length === 2) {
                        canvas2d.lineTo(points[1].x, points[1].y);
                    } else {
                        for (let i = 1; i < points.length - 1; i++) {
                            const xc = (points[i].x + points[i + 1].x) / 2;
                            const yc = (points[i].y + points[i + 1].y) / 2;
                            canvas2d.quadraticCurveTo(points[i].x, points[i].y, xc, yc);
                        }
                        // Last segment
                        canvas2d.lineTo(points[points.length - 1].x, points[points.length - 1].y);
                    }

                    canvas2d.stroke();
                    canvas2d.restore();
                } catch (err) {
                    console.error("Canvas2D描画エラー:", err);
                    setError(err instanceof Error ? err.message : "Canvas2D描画中にエラーが発生しました");
                }
            }
        },
        [isWasmReady, canvas2d]
    );

    // クリア関数
    const clear = useCallback(async () => {
        try {
            if (isWasmReady && engineRef.current) {
                await engineRef.current.clear();
                // エラー状態もクリア
                setError(null);
            } else if (canvas2d && canvasRef.current) {
                canvas2d.save();
                canvas2d.clearRect(0, 0, canvasRef.current.width, canvasRef.current.height);
                // 白背景にリセット
                canvas2d.fillStyle = "#ffffff";
                canvas2d.fillRect(0, 0, canvasRef.current.width, canvasRef.current.height);
                canvas2d.restore();
                // エラー状態もクリア
                setError(null);
            }
        } catch (err) {
            console.error("クリアエラー:", err);
            setError(err instanceof Error ? err.message : "キャンバスのクリア中にエラーが発生しました");
        }
    }, [isWasmReady, canvas2d]);

    return {
        canvasRef,
        isReady: !isInitializing && !!(canvas2d || isWasmReady),
        isWasmReady,
        drawStroke,
        clear,
        engine: engineRef.current,
        error,
        renderMode: isWasmReady ? "wasm" : "canvas2d",
        isInitializing,
    };
};
