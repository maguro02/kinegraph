import { useRef, useCallback, useEffect, useState } from "react";
import { useAtom } from "jotai";
import { commands, binaryCommands, simpleBinaryCommands, DrawEngineCommand, BrushSettings } from "./bindings";
import { drawingEngineStateAtom } from "../store/atoms";
import { BinaryProtocol, PerformanceMeasure } from "./binaryProtocol";
import { SimpleBinaryProtocol } from "./simpleBinaryProtocol";
import { DrawCommandBatcher, DrawCommandBatch, optimizeCommands } from "./drawCommandBatcher";

/**
 * 描画エンジンフック
 *
 * @param options - オプション設定
 * @param options.useBinary - バイナリ送信モードを有効にする（デフォルト: false）
 *
 * @example
 * // 通常のJSON送信モード
 * const { canvasRef, beginStroke, continueStroke, endStroke } = useDrawingEngine();
 *
 * @example
 * // バイナリ送信モード（高パフォーマンス）
 * const { canvasRef, beginStroke, continueStroke, endStroke } = useDrawingEngine({ useBinary: true });
 */

export function useDrawingEngine(options?: {
    useBinary?: boolean;
    updateInterval?: number; // キャンバス更新間隔（ms）
    moveEventThrottle?: number; // pointerMoveイベントの間引き回数（デフォルト: 2 = 2回に1回処理）
    distanceThreshold?: number; // 移動距離の閾値（ピクセル）
    debugMode?: boolean; // デバッグログの有効/無効
}) {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const [state, setState] = useAtom(drawingEngineStateAtom);
    const batcherRef = useRef<DrawCommandBatcher | null>(null);
    const [useBatching, setUseBatching] = useState(true); // バッチ処理の有効/無効を切り替え
    const [useBinary, setUseBinary] = useState(options?.useBinary ?? false); // バイナリ送信モード
    const updateInterval = options?.updateInterval ?? 33; // デフォルト33ms（30fps）
    const moveEventThrottle = options?.moveEventThrottle ?? 2; // デフォルト: 2回に1回処理
    const distanceThreshold = options?.distanceThreshold ?? 2; // デフォルト: 2ピクセル
    const debugMode = options?.debugMode ?? false; // デバッグモード

    // バイナリコマンドの送信
    const sendBinaryCommand = useCallback(
        async (command: DrawEngineCommand) => {
            if (!state.canvasId || !state.isInitialized) {
                throw new Error("Drawing engine not initialized");
            }

            try {
                // シンプルバイナリプロトコルを使用
                const buffer = SimpleBinaryProtocol.encodeCommand(command);
                if (debugMode) console.log(`[TS] Sending simple binary command, size: ${buffer.length} bytes`);
                await simpleBinaryCommands.drawCommand(buffer);
            } catch (error) {
                console.error("Failed to send binary command:", error);
                // フォールバック: JSONで送信
                console.warn("[TS] Falling back to JSON mode");
                const result = await commands.drawCommand(command);
                if (result.status === "error") {
                    throw new Error(result.error);
                }
            }
        },
        [state.canvasId, state.isInitialized, debugMode]
    );

    // 描画エンジンの初期化
    const initEngine = useCallback(async (width: number, height: number) => {
        try {
            const result = await commands.initCanvas(width, height);
            if (result.status === "ok") {
                setState({
                    canvasId: result.data,
                    isInitialized: true,
                    error: null,
                });
                return result.data;
            } else {
                setState((prev: typeof state) => ({
                    ...prev,
                    error: result.error,
                }));
                throw new Error(result.error);
            }
        } catch (error) {
            setState((prev: typeof state) => ({
                ...prev,
                error: error instanceof Error ? error.message : String(error),
            }));
            throw error;
        }
    }, []);

    // バッチフラッシュハンドラー
    const handleBatchFlush = useCallback(
        async (batch: DrawCommandBatch) => {
            if (!state.canvasId || !state.isInitialized) {
                throw new Error("Drawing engine not initialized");
            }

            // コマンドを最適化
            const optimizedCommands = optimizeCommands(batch.commands);

            // バイナリモードの場合
            if (useBinary) {
                // シンプルバイナリプロトコルでバッチ送信
                try {
                    const buffer = SimpleBinaryProtocol.encodeCommandBatch(optimizedCommands);
                    if (debugMode) {
                        console.log(
                            `[TS] Sending simple binary batch, size: ${buffer.length} bytes for ${optimizedCommands.length} commands`
                        );
                    }
                    await simpleBinaryCommands.drawBatch(buffer);
                } catch (error) {
                    console.error("Failed to send binary batch:", error);
                    // フォールバック: JSONで個別送信
                    console.warn(`[TS] Falling back to JSON mode for ${optimizedCommands.length} commands`);
                    for (const command of optimizedCommands) {
                        const result = await commands.drawCommand(command);
                        if (result.status === "error") {
                            throw new Error(result.error);
                        }
                    }
                }
            } else {
                // 通常のJSON送信
                for (const command of optimizedCommands) {
                    const result = await commands.drawCommand(command);
                    if (result.status === "error") {
                        throw new Error(result.error);
                    }
                }
            }
        },
        [state.canvasId, state.isInitialized, useBinary, sendBinaryCommand, debugMode]
    );

    // バッチャーの初期化
    useEffect(() => {
        if (useBatching && state.isInitialized) {
            batcherRef.current = new DrawCommandBatcher(handleBatchFlush, {
                maxBatchSize: 10, // ストローク中のレスポンスを向上させるため、バッチサイズを小さく
                flushInterval: 4, // 4ms（約250fps相当）でより頻繁にフラッシュ
                priorityThreshold: 5, // 優先度5以上は即座に送信
            });
        } else {
            if (batcherRef.current) {
                batcherRef.current.destroy();
                batcherRef.current = null;
            }
        }

        return () => {
            if (batcherRef.current) {
                batcherRef.current.destroy();
                batcherRef.current = null;
            }
        };
    }, [useBatching, state.isInitialized, handleBatchFlush]);

    // 描画コマンドの送信
    const sendCommand = useCallback(
        async (command: DrawEngineCommand) => {
            if (!state.canvasId || !state.isInitialized) {
                throw new Error("Drawing engine not initialized");
            }

            console.log("[DEBUG] sendCommand called with:", Object.keys(command)[0]);

            // バイナリモードの場合
            if (useBinary) {
                // バッチ処理が有効な場合でも、バイナリモードではバッチャーに追加
                if (useBatching && batcherRef.current) {
                    console.log("[DEBUG] Adding command to batcher (binary mode)");
                    batcherRef.current.add(command);
                    return;
                }
                // 直接バイナリ送信
                console.log("[DEBUG] Sending command directly (binary mode)");
                await sendBinaryCommand(command);
                return;
            }

            // 通常のJSON送信
            if (useBatching && batcherRef.current) {
                console.log("[DEBUG] Adding command to batcher (JSON mode)");
                batcherRef.current.add(command);
                return;
            }

            // バッチ処理が無効な場合は直接送信
            console.log("[DEBUG] Sending command directly (JSON mode)");
            const result = await commands.drawCommand(command);
            if (result.status === "error") {
                throw new Error(result.error);
            }
        },
        [state.canvasId, state.isInitialized, useBatching, useBinary, sendBinaryCommand]
    );

    // レンダリング結果の取得と表示
    const updateCanvas = useCallback(
        async (useDiff: boolean = true, forceFullUpdate: boolean = false) => {
            console.log(`[DEBUG] updateCanvas called with useDiff=${useDiff}, forceFullUpdate=${forceFullUpdate}`);

            if (!state.canvasId || !canvasRef.current) {
                console.log("[DEBUG] updateCanvas: No canvasId or canvasRef, returning");
                return;
            }

            const ctx = canvasRef.current.getContext("2d");
            if (!ctx) {
                console.log("[DEBUG] updateCanvas: No 2D context, returning");
                return;
            }

            try {
                if (useDiff && !forceFullUpdate) {
                    console.log("[DEBUG] updateCanvas: Using diff update");
                    // 差分更新を使用
                    const result = await commands.getDiffRenderResult(state.canvasId);

                    if (result.status === "ok" && result.data.dirtyRegion) {
                        const region = result.data.dirtyRegion;
                        const imageData = new ImageData(
                            new Uint8ClampedArray(region.imageData.data),
                            region.imageData.width,
                            region.imageData.height
                        );
                        ctx.putImageData(imageData, region.x, region.y);
                        console.log(
                            `[DEBUG] Updated canvas with diff region at (${region.x}, ${region.y}) with size ${region.imageData.width}x${region.imageData.height}`
                        );
                    } else if (result.status === "ok" && !result.data.dirtyRegion) {
                        // dirtyRegionがnullの場合、全体更新にフォールバック
                        console.log("[DEBUG] No dirty region, falling back to full update");
                        await updateCanvas(false);
                    }
                } else {
                    console.log("[DEBUG] updateCanvas: Using full update");
                    // 全体更新を使用
                    const result = await commands.getRenderResult(state.canvasId);
                    console.log("[DEBUG] getRenderResult returned:", result.status);
                    if (result.status === "ok") {
                        const renderData = result.data;
                        const imageData = new ImageData(
                            new Uint8ClampedArray(renderData.imageData.data),
                            renderData.imageData.width,
                            renderData.imageData.height
                        );
                        ctx.putImageData(imageData, 0, 0);
                        console.log(
                            `[DEBUG] Successfully updated canvas with full render data of size ${renderData.imageData.width}x${renderData.imageData.height}`
                        );
                    } else {
                        console.error("[DEBUG] getRenderResult failed:", result);
                    }
                }
            } catch (error) {
                console.error("[DEBUG] Failed to update canvas:", error);
            }
        },
        [state.canvasId]
    );

    // キャンバスのリサイズ
    const resizeCanvas = useCallback(
        async (width: number, height: number) => {
            if (!state.canvasId) {
                throw new Error("Canvas not initialized");
            }

            const result = await commands.resizeCanvas(state.canvasId, width, height);
            if (result.status === "error") {
                throw new Error(result.error);
            }
        },
        [state.canvasId]
    );

    // ブラシ設定のヘルパー関数
    const setBrush = useCallback(
        async (settings: BrushSettings) => {
            await sendCommand({ setBrush: settings });
        },
        [sendCommand]
    );

    // ストローク操作のヘルパー関数（エラーハンドリング付き）
    const beginStroke = useCallback(
        async (x: number, y: number, pressure: number = 1.0) => {
            try {
                await sendCommand({ beginStroke: { x, y, pressure } });
            } catch (error) {
                console.error("Failed to begin stroke:", error);
                setState((prev: typeof state) => ({
                    ...prev,
                    error: error instanceof Error ? error.message : "Failed to begin stroke",
                }));
                throw error;
            }
        },
        [sendCommand, setState]
    );

    const continueStroke = useCallback(
        async (x: number, y: number, pressure: number = 1.0) => {
            try {
                console.log(
                    `[DEBUG] continueStroke called at (${x.toFixed(2)}, ${y.toFixed(
                        2
                    )}) with pressure ${pressure.toFixed(2)}`
                );
                await sendCommand({ continueStroke: { x, y, pressure } });
            } catch (error) {
                console.error("Failed to continue stroke:", error);
                setState((prev: typeof state) => ({
                    ...prev,
                    error: error instanceof Error ? error.message : "Failed to continue stroke",
                }));
                throw error;
            }
        },
        [sendCommand, setState]
    );

    const endStroke = useCallback(async () => {
        try {
            await sendCommand("endStroke");
        } catch (error) {
            console.error("Failed to end stroke:", error);
            setState((prev: typeof state) => ({
                ...prev,
                error: error instanceof Error ? error.message : "Failed to end stroke",
            }));
            throw error;
        }
    }, [sendCommand, setState]);

    // レイヤー操作のヘルパー関数
    const createLayer = useCallback(async () => {
        await sendCommand("createLayer");
    }, [sendCommand]);

    const deleteLayer = useCallback(
        async (index: number) => {
            await sendCommand({ deleteLayer: index });
        },
        [sendCommand]
    );

    const setActiveLayer = useCallback(
        async (index: number) => {
            await sendCommand({ setActiveLayer: index });
        },
        [sendCommand]
    );

    // キャンバスクリア
    const clearCanvas = useCallback(async () => {
        await sendCommand("clear");
    }, [sendCommand]);

    // ポインターイベントハンドラー
    const [isDrawing, setIsDrawing] = useState(false);
    const updatePendingRef = useRef(false);
    const lastUpdateTimeRef = useRef(0);
    const moveEventCountRef = useRef(0); // pointerMoveイベントカウンター
    const lastPositionRef = useRef<{ x: number; y: number } | null>(null); // 最後の描画位置
    const boundingRectCacheRef = useRef<DOMRect | null>(null); // getBoundingClientRect()のキャッシュ
    const rectCacheTimeRef = useRef(0); // キャッシュのタイムスタンプ

    const handlePointerDown = useCallback(
        async (e: React.PointerEvent<HTMLCanvasElement>) => {
            if (!canvasRef.current) return;

            // getBoundingClientRect()のキャッシュを更新
            boundingRectCacheRef.current = canvasRef.current.getBoundingClientRect();
            rectCacheTimeRef.current = performance.now();

            const rect = boundingRectCacheRef.current;
            const x = e.clientX - rect.left;
            const y = e.clientY - rect.top;
            const pressure = e.pressure || 1.0;

            setIsDrawing(true);

            // ストローク開始時の位置を記録
            lastPositionRef.current = { x, y };
            moveEventCountRef.current = 0;

            // ストローク開始時にタイマーをリセット
            lastUpdateTimeRef.current = 0;

            // ストローク開始時はバッチ処理を使わず直接送信
            if (useBinary) {
                if (debugMode) console.log("[DEBUG] Sending beginStroke via binary protocol");
                await sendBinaryCommand({ beginStroke: { x, y, pressure } });
            } else if (useBatching && batcherRef.current) {
                if (debugMode) console.log("[DEBUG] Sending beginStroke via JSON (batching enabled but bypassed)");
                const result = await commands.drawCommand({ beginStroke: { x, y, pressure } });
                if (result.status === "error") {
                    throw new Error(result.error);
                }
            } else {
                await beginStroke(x, y, pressure);
            }

            // ストローク開始時は全体更新を使用
            await updateCanvas(false);
        },
        [beginStroke, sendCommand, updateCanvas, useBatching, useBinary, sendBinaryCommand, debugMode]
    );

    const handlePointerMove = useCallback(
        async (e: React.PointerEvent<HTMLCanvasElement>) => {
            if (!isDrawing || !canvasRef.current) return;

            // イベント間引き処理
            moveEventCountRef.current++;
            if (moveEventCountRef.current % moveEventThrottle !== 0) {
                return; // 指定回数に1回だけ処理
            }

            // getBoundingClientRect()のキャッシュを使用（500ms有効）
            const now = performance.now();
            if (!boundingRectCacheRef.current || now - rectCacheTimeRef.current > 500) {
                boundingRectCacheRef.current = canvasRef.current.getBoundingClientRect();
                rectCacheTimeRef.current = now;
            }

            const rect = boundingRectCacheRef.current;
            const x = e.clientX - rect.left;
            const y = e.clientY - rect.top;
            const pressure = e.pressure || 1.0;

            // 距離ベースの間引き
            if (lastPositionRef.current) {
                const dx = x - lastPositionRef.current.x;
                const dy = y - lastPositionRef.current.y;
                const distance = Math.sqrt(dx * dx + dy * dy);

                if (distance < distanceThreshold) {
                    return; // 移動距離が闾値未満なら処理しない
                }
            }

            // 現在位置を記録
            lastPositionRef.current = { x, y };

            if (debugMode) {
                console.log(`[DEBUG] handlePointerMove: Drawing at (${x.toFixed(2)}, ${y.toFixed(2)})`);
            }

            // ストローク中はバッチ処理を使わず直接送信
            // バイナリモードの場合は、sendBinaryCommandを直接使用
            if (useBinary) {
                if (debugMode) console.log("[DEBUG] Sending continueStroke via binary protocol");
                await sendBinaryCommand({ continueStroke: { x, y, pressure } });
            } else if (useBatching && batcherRef.current) {
                if (debugMode) console.log("[DEBUG] Sending continueStroke via JSON (batching enabled but bypassed)");
                // バッチ処理が有効でも、continueStrokeは直接送信
                const result = await commands.drawCommand({ continueStroke: { x, y, pressure } });
                if (result.status === "error") {
                    throw new Error(result.error);
                }
            } else {
                if (debugMode) console.log("[DEBUG] Sending continueStroke directly (batching disabled)");
                await continueStroke(x, y, pressure);
            }

            // requestAnimationFrameベースの更新制御
            const timeSinceLastUpdate = now - lastUpdateTimeRef.current;

            // 最初の更新または前回の更新から指定間隔以上経過した場合のみ更新
            if (lastUpdateTimeRef.current === 0 || timeSinceLastUpdate >= updateInterval) {
                if (debugMode) {
                    console.log("[DEBUG] Updating canvas (time since last: " + timeSinceLastUpdate + "ms)");
                }
                lastUpdateTimeRef.current = now;

                // 既に更新が予約されていない場合のみ実行
                if (!updatePendingRef.current) {
                    updatePendingRef.current = true;

                    // requestAnimationFrameを使用して更新をスケジュール
                    requestAnimationFrame(() => {
                        updateCanvas(false)
                            .then(() => {
                                updatePendingRef.current = false;
                            })
                            .catch((error) => {
                                console.error("[DEBUG] Canvas update failed:", error);
                                updatePendingRef.current = false;
                            });
                    });
                }
            } else if (debugMode) {
                console.log("[DEBUG] Skipping canvas update (too soon, " + timeSinceLastUpdate + "ms since last)");
            }
        },
        [
            isDrawing,
            continueStroke,
            sendCommand,
            updateCanvas,
            useBatching,
            useBinary,
            sendBinaryCommand,
            updateInterval,
            moveEventThrottle,
            distanceThreshold,
            debugMode,
        ]
    );

    const handlePointerUp = useCallback(async () => {
        if (!isDrawing) return;

        if (debugMode) console.log("[DEBUG] handlePointerUp: Ending stroke");
        setIsDrawing(false);

        // ポインター状態をリセット
        lastPositionRef.current = null;
        moveEventCountRef.current = 0;

        // ストローク終了時はバッチ処理を使わず直接送信
        if (useBinary) {
            if (debugMode) console.log("[DEBUG] Sending endStroke via binary protocol");
            // バッチャーに残っているコマンドがあればフラッシュ
            if (useBatching && batcherRef.current) {
                await batcherRef.current.flush();
            }
            await sendBinaryCommand("endStroke");
        } else if (useBatching && batcherRef.current) {
            if (debugMode) console.log("[DEBUG] Flushing batcher and sending endStroke via JSON");
            // 残っているコマンドがあればフラッシュ
            await batcherRef.current.flush();
            // endStrokeは直接送信
            const result = await commands.drawCommand("endStroke");
            if (result.status === "error") {
                throw new Error(result.error);
            }
        } else {
            if (debugMode) console.log("[DEBUG] Sending endStroke directly");
            await endStroke();
        }

        // ストローク終了時は差分更新を使用可能
        if (debugMode) console.log("[DEBUG] Updating canvas on stroke end");
        await updateCanvas(true);
        if (debugMode) console.log("[DEBUG] handlePointerUp completed");
    }, [isDrawing, endStroke, sendCommand, updateCanvas, useBatching, useBinary, sendBinaryCommand, debugMode]);

    // バイナリ転送を使用したキャンバスデータの取得
    const fetchCanvasDataBinary = useCallback(
        async (options?: { compress?: boolean; measurePerformance?: boolean }) => {
            if (!state.canvasId) {
                throw new Error("Canvas not initialized");
            }

            const perf = options?.measurePerformance ? new PerformanceMeasure() : null;
            perf?.mark("start");

            try {
                // バイナリストリーミングオプション
                const streamingOptions = {
                    compress: options?.compress ?? false,
                    chunkSize: undefined,
                    useStreaming: true,
                };

                perf?.mark("invoke_start");
                // Tauriコマンドを呼び出してバイナリデータを取得
                const binaryData = await binaryCommands.getCanvasDataBinaryStream(state.canvasId, streamingOptions);
                const buffer = binaryData.buffer;
                perf?.mark("invoke_end");

                // バイナリプロトコルでデコード
                perf?.mark("decode_start");
                const { width, height, data } = await BinaryProtocol.decodeAndDecompressImage(buffer);
                perf?.mark("decode_end");

                // キャンバスに描画
                if (canvasRef.current) {
                    const ctx = canvasRef.current.getContext("2d");
                    if (ctx) {
                        perf?.mark("render_start");
                        const imageData = new ImageData(new Uint8ClampedArray(data), width, height);
                        ctx.putImageData(imageData, 0, 0);
                        perf?.mark("render_end");
                    }
                }

                if (perf) {
                    const report = perf.getReport();
                    console.log("Binary transfer performance:", report);
                }

                return { width, height, data };
            } catch (error) {
                console.error("Failed to fetch canvas data (binary):", error);
                throw error;
            }
        },
        [state.canvasId]
    );

    // パフォーマンステスト用の比較関数
    const compareTransferMethods = useCallback(async () => {
        if (!state.canvasId) {
            throw new Error("Canvas not initialized");
        }

        console.log("=== Performance Comparison ===");

        // バッチ処理の統計情報を表示
        if (batcherRef.current) {
            const stats = batcherRef.current.getStats();
            console.log("Batch stats:", stats);
        }

        // JSON転送のテスト
        const jsonStart = performance.now();
        try {
            await updateCanvas(false); // 全体更新を使用
            const jsonTime = performance.now() - jsonStart;
            console.log(`JSON transfer: ${jsonTime.toFixed(2)}ms`);
        } catch (error) {
            console.error("JSON transfer failed:", error);
        }

        // バイナリ転送のテスト（非圧縮）
        const binaryStart = performance.now();
        try {
            await fetchCanvasDataBinary({ compress: false, measurePerformance: true });
            const binaryTime = performance.now() - binaryStart;
            console.log(`Binary transfer (uncompressed): ${binaryTime.toFixed(2)}ms`);
        } catch (error) {
            console.error("Binary transfer failed:", error);
        }

        // バイナリ転送のテスト（圧縮）
        const binaryCompressedStart = performance.now();
        try {
            await fetchCanvasDataBinary({ compress: true, measurePerformance: true });
            const binaryCompressedTime = performance.now() - binaryCompressedStart;
            console.log(`Binary transfer (compressed): ${binaryCompressedTime.toFixed(2)}ms`);
        } catch (error) {
            console.error("Binary compressed transfer failed:", error);
        }

        // バッチ処理有効/無効の比較
        console.log(`Batching enabled: ${useBatching}`);
        console.log(`Binary mode enabled: ${useBinary}`);
    }, [state.canvasId, updateCanvas, fetchCanvasDataBinary, useBatching, useBinary]);

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
        // バイナリ転送
        fetchCanvasDataBinary,
        compareTransferMethods,
        // バッチ処理制御
        useBatching,
        setUseBatching,
        batcherStats: batcherRef.current?.getStats(),
        // バイナリモード制御
        useBinary,
        setUseBinary,
    };
}
