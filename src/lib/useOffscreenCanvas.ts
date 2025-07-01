/**
 * OffscreenCanvas を使用した高速描画フック
 * Web Worker で描画処理を実行し、メインスレッドの負荷を軽減
 */

import { useEffect, useRef, useCallback, useState } from 'react';
import type { DrawCommand } from './offscreenCanvasWorker';

interface UseOffscreenCanvasOptions {
    width: number;
    height: number;
    onReady?: () => void;
}

interface StrokePoint {
    x: number;
    y: number;
    pressure: number;
}

export function useOffscreenCanvas(options: UseOffscreenCanvasOptions) {
    const { width, height, onReady } = options;
    const [isReady, setIsReady] = useState(false);
    const workerRef = useRef<Worker | null>(null);
    const canvasRef = useRef<HTMLCanvasElement | null>(null);
    const currentStrokeRef = useRef<StrokePoint[]>([]);
    const strokeBufferRef = useRef<StrokePoint[]>([]);
    const rafIdRef = useRef<number | null>(null);

    // Worker の初期化
    useEffect(() => {
        if (!workerRef.current && typeof Worker !== 'undefined') {
            const worker = new Worker(
                new URL('./offscreenCanvasWorker.ts', import.meta.url),
                { type: 'module' }
            );

            worker.onmessage = (event) => {
                const { type, bitmap, rect } = event.data;

                switch (type) {
                    case 'initialized':
                        setIsReady(true);
                        onReady?.();
                        break;

                    case 'strokeUpdate':
                        // Worker から受け取った ImageBitmap を Canvas に描画
                        if (canvasRef.current && bitmap) {
                            const ctx = canvasRef.current.getContext('2d');
                            if (ctx) {
                                ctx.drawImage(bitmap, rect.x, rect.y);
                            }
                        }
                        break;

                    case 'updated':
                        // Rust エンジンからの更新完了通知
                        console.debug('[OffscreenCanvas] 更新完了:', rect);
                        break;
                }
            };

            worker.onerror = (error) => {
                console.error('[OffscreenCanvas] Worker エラー:', error);
            };

            workerRef.current = worker;

            // Worker を初期化
            worker.postMessage({
                type: 'init',
                data: { width, height }
            } as DrawCommand);
        }

        return () => {
            if (workerRef.current) {
                workerRef.current.terminate();
                workerRef.current = null;
                setIsReady(false);
            }
        };
    }, [width, height, onReady]);

    // Canvas 参照を設定
    const setCanvasRef = useCallback((canvas: HTMLCanvasElement | null) => {
        canvasRef.current = canvas;
    }, []);

    // ストローク開始
    const beginStroke = useCallback((x: number, y: number, color: string, size: number) => {
        if (!isReady || !workerRef.current) return;

        currentStrokeRef.current = [{ x, y, pressure: 1.0 }];
        strokeBufferRef.current = [{ x, y, pressure: 1.0 }];

        // 即座に点を描画
        workerRef.current.postMessage({
            type: 'stroke',
            data: {
                points: currentStrokeRef.current,
                color,
                size
            }
        } as DrawCommand);
    }, [isReady]);

    // ストローク点を追加（バッファリング付き）
    const addStrokePoint = useCallback((x: number, y: number, pressure: number = 1.0) => {
        if (!isReady || !workerRef.current) return;

        strokeBufferRef.current.push({ x, y, pressure });

        // requestAnimationFrame でバッチ処理
        if (!rafIdRef.current) {
            rafIdRef.current = requestAnimationFrame(() => {
                if (workerRef.current && strokeBufferRef.current.length > 0) {
                    // バッファ内の点を一度に送信
                    workerRef.current.postMessage({
                        type: 'stroke',
                        data: {
                            points: [...strokeBufferRef.current],
                            color: '#000000', // TODO: 実際の色を保持
                            size: 5 // TODO: 実際のサイズを保持
                        }
                    } as DrawCommand);

                    // 現在のストロークに追加
                    currentStrokeRef.current.push(...strokeBufferRef.current);
                    strokeBufferRef.current = [];
                }
                rafIdRef.current = null;
            });
        }
    }, [isReady]);

    // ストローク完了
    const completeStroke = useCallback(() => {
        if (!isReady || !workerRef.current) return;

        // 残りのバッファを送信
        if (strokeBufferRef.current.length > 0 && rafIdRef.current) {
            cancelAnimationFrame(rafIdRef.current);
            rafIdRef.current = null;

            workerRef.current.postMessage({
                type: 'stroke',
                data: {
                    points: [...strokeBufferRef.current],
                    color: '#000000',
                    size: 5
                }
            } as DrawCommand);
        }

        currentStrokeRef.current = [];
        strokeBufferRef.current = [];
    }, [isReady]);

    // Rust エンジンからの更新を適用
    const applyRustUpdate = useCallback((imageData: ArrayBuffer, rect: { x: number; y: number; width: number; height: number }) => {
        if (!isReady || !workerRef.current) return;

        workerRef.current.postMessage({
            type: 'update',
            data: { imageData, rect }
        } as DrawCommand);
    }, [isReady]);

    // Canvas をクリア
    const clearCanvas = useCallback(() => {
        if (!isReady || !workerRef.current) return;

        workerRef.current.postMessage({
            type: 'clear',
            data: null
        } as DrawCommand);
    }, [isReady]);

    return {
        isReady,
        setCanvasRef,
        beginStroke,
        addStrokePoint,
        completeStroke,
        applyRustUpdate,
        clearCanvas
    };
}