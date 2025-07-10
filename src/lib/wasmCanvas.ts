import type { DrawingContext, Point, StrokeId, Color, BrushTool } from "../types/drawing.ts";

// Worker通信用のレスポンスタイプ
type DrawEngineResponse = {
    type: string;
    data?: any;
    id?: string;
};

export class WasmCanvasDrawingContext implements DrawingContext {
    private worker: Worker | null = null;
    private pendingMessages = new Map<string, { resolve: Function; reject: Function }>();
    private initialized = false;
    private sharedBuffer: SharedArrayBuffer | null = null;
    private canvasCtx: CanvasRenderingContext2D | null = null;

    // パフォーマンス最適化用
    private imageData: ImageData | null = null;
    private clampedArray: Uint8ClampedArray | null = null;
    private sharedView: Uint8Array | null = null; // 再利用可能なビュー
    private dirtyRegion: {
        x: number;
        y: number;
        width: number;
        height: number;
        isPartialUpdate?: boolean;
    } | null = null;
    private renderScheduled = false;
    private lastRenderTime = 0;
    private readonly MIN_RENDER_INTERVAL = 16; // 60fps

    constructor(private canvas: HTMLCanvasElement, private width: number, private height: number) {}

    async initialize(): Promise<void> {
        if (this.initialized) return;

        try {
            // Canvas 2Dコンテキストを取得
            this.canvasCtx = this.canvas.getContext("2d", {
                alpha: false, // 透過不要の場合はパフォーマンス向上
                desynchronized: true, // 非同期レンダリング
            });
            if (!this.canvasCtx) {
                throw new Error("Failed to get 2D context");
            }

            // ImageDataを事前に作成してメモリアロケーションを削減
            this.imageData = this.canvasCtx.createImageData(this.width, this.height);
            this.clampedArray = this.imageData.data;

            // Workerを作成
            this.worker = new Worker("/js/wasmDrawEngineWorker.js", { type: "module" });

            // Workerメッセージハンドラを設定
            this.worker.addEventListener("message", (event: MessageEvent<DrawEngineResponse>) => {
                const response = event.data;

                // SharedArrayBufferの処理
                if (response.type === "initialized" && response.data?.sharedBuffer) {
                    this.handleSharedBuffer(response.data.sharedBuffer);
                }

                // レンダリング更新の処理（差分情報付き）
                if (response.type === "rendered") {
                    this.scheduleCanvasUpdate(response.data.dirtyRegion);
                }

                // ペンディングメッセージの処理
                if (response.id) {
                    const pending = this.pendingMessages.get(response.id);
                    if (pending) {
                        this.pendingMessages.delete(response.id);
                        if (response.type === "error") {
                            pending.reject(new Error(response.data?.message || "Unknown error"));
                        } else {
                            pending.resolve(response.data);
                        }
                    }
                }
            });

            // Workerを初期化
            console.log("Initializing WASM worker with:", { width: this.width, height: this.height });

            this.worker.postMessage({
                type: "init",
                data: {
                    width: this.width,
                    height: this.height,
                    enableIncrementalRendering: true, // 差分レンダリングを有効化
                },
            });

            // 初期化完了を待つ
            await new Promise<void>((resolve, reject) => {
                const timeout = setTimeout(() => {
                    reject(new Error("Worker initialization timeout"));
                }, 10000);

                const handler = (event: MessageEvent) => {
                    if (event.data.type === "initialized") {
                        clearTimeout(timeout);
                        this.worker!.removeEventListener("message", handler);
                        this.initialized = true;
                        resolve();
                    } else if (event.data.type === "error" && event.data.data?.originalType === "init") {
                        clearTimeout(timeout);
                        this.worker!.removeEventListener("message", handler);
                        reject(new Error(event.data.data.message));
                    }
                };

                this.worker!.addEventListener("message", handler);
            });

            console.log("WASM worker initialized successfully");
        } catch (error) {
            console.error("Failed to initialize WASM canvas:", error);
            throw error;
        }
    }

    private async sendMessage(message: any): Promise<any> {
        if (!this.worker) {
            throw new Error("Worker not initialized");
        }

        // バッチ処理のため、addPointは非同期で送信
        if (message.type === "addPoint") {
            this.worker!.postMessage(message);
            return Promise.resolve();
        }

        // その他のメッセージは同期的に処理
        this.worker!.postMessage(message);

        if (message.type === "endStroke" || message.type === "clear") {
            return Promise.resolve();
        }

        if (message.type === "startStroke") {
            return new Promise((resolve, reject) => {
                const handler = (event: MessageEvent) => {
                    if (event.data.type === "strokeStarted") {
                        this.worker!.removeEventListener("message", handler);
                        resolve(event.data.data);
                    } else if (event.data.type === "error" && event.data.data?.originalType === "startStroke") {
                        this.worker!.removeEventListener("message", handler);
                        reject(new Error(event.data.data.message));
                    }
                };
                this.worker!.addEventListener("message", handler);

                setTimeout(() => {
                    this.worker!.removeEventListener("message", handler);
                    reject(new Error("startStroke timeout"));
                }, 1000);
            });
        }

        return Promise.resolve();
    }

    async startStroke(point: Point, tool: BrushTool, color: Color): Promise<StrokeId> {
        if (!this.initialized) {
            throw new Error("WasmCanvas not initialized");
        }

        const strokeId = `stroke_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`;

        console.log("WasmCanvas startStroke:", { point, tool, color, strokeId });

        await this.sendMessage({
            type: "startStroke",
            data: {
                point,
                tool,
                color,
                strokeId,
            },
        });

        return strokeId;
    }

    async addPoint(strokeId: StrokeId, point: Point): Promise<void> {
        if (!this.initialized) {
            throw new Error("WasmCanvas not initialized");
        }

        // 非同期で送信（バッチ処理対応）
        await this.sendMessage({
            type: "addPoint",
            data: {
                strokeId,
                point,
            },
        });
    }

    async endStroke(strokeId: StrokeId): Promise<void> {
        await this.sendMessage({
            type: "endStroke",
            data: {
                strokeId,
            },
        });
    }

    async clear(): Promise<void> {
        await this.sendMessage({
            type: "clear",
            data: {},
        });
    }

    resize(width: number, height: number): void {
        this.width = width;
        this.height = height;

        // ImageDataを再作成
        if (this.canvasCtx) {
            this.imageData = this.canvasCtx.createImageData(width, height);
            this.clampedArray = this.imageData.data;
        }

        if (this.worker) {
            this.worker.postMessage({
                type: "resize",
                data: { width, height },
            });
        }
    }

    destroy(): void {
        if (this.worker) {
            this.worker.terminate();
            this.worker = null;
        }
        this.pendingMessages.clear();
        this.initialized = false;
        this.sharedBuffer = null;
        this.sharedView = null;
        this.canvasCtx = null;
        this.imageData = null;
        this.clampedArray = null;
    }

    private handleSharedBuffer(buffer: SharedArrayBuffer): void {
        this.sharedBuffer = buffer;
    }

    private scheduleCanvasUpdate(dirtyRegion?: any): void {
        if (dirtyRegion) {
            this.dirtyRegion = dirtyRegion;
        }
        console.log("scheduleCanvasUpdate called", {
            dirtyRegion: this.dirtyRegion,
            renderScheduled: this.renderScheduled,
            lastRenderTime: this.lastRenderTime,
            MIN_RENDER_INTERVAL: this.MIN_RENDER_INTERVAL,
        });
        if (this.renderScheduled) return;

        const now = performance.now();
        const timeSinceLastRender = now - this.lastRenderTime;

        if (timeSinceLastRender >= this.MIN_RENDER_INTERVAL) {
            // 即座にレンダリング
            this.updateCanvas();
        } else {
            // 次のフレームでレンダリング
            this.renderScheduled = true;
            requestAnimationFrame(() => {
                this.renderScheduled = false;
                this.updateCanvas();
            });
        }
    }

    private updateCanvas(): void {
        if (!this.sharedBuffer || !this.canvasCtx || !this.clampedArray || !this.imageData) return;

        this.lastRenderTime = performance.now();

        try {
            // SharedArrayBufferから直接Uint8ClampedArrayにコピー
            const sharedView = new Uint8Array(this.sharedBuffer);

            if (this.dirtyRegion && this.dirtyRegion.isPartialUpdate) {
                // 部分更新の場合
                const region = this.dirtyRegion;
                const x = Math.floor(region.x);
                const y = Math.floor(region.y);
                const width = Math.ceil(region.width);
                const height = Math.ceil(region.height);

                // 境界チェック
                if (x >= 0 && y >= 0 && x + width <= this.width && y + height <= this.height) {
                    // 部分的なImageDataを作成
                    const partialImageData = this.canvasCtx.createImageData(width, height);
                    const partialArray = partialImageData.data;

                    // 該当領域のピクセルデータをコピー
                    for (let dy = 0; dy < height; dy++) {
                        const srcY = y + dy;
                        const srcOffset = (srcY * this.width + x) * 4;
                        const dstOffset = dy * width * 4;

                        // 1行分のデータをコピー
                        for (let dx = 0; dx < width * 4; dx++) {
                            partialArray[dstOffset + dx] = sharedView[srcOffset + dx];
                        }
                    }

                    // 部分的に描画
                    this.canvasCtx.putImageData(partialImageData, x, y);

                    // デバッグ用: 更新領域を視覚化（開発時のみ）
                    if (process.env.NODE_ENV === "development") {
                        this.visualizeDirtyRegion(region);
                    }
                } else {
                    // 境界外の場合はフルレンダリング
                    this.clampedArray.set(sharedView);
                    this.canvasCtx.putImageData(this.imageData, 0, 0);
                }
            } else {
                // 全体レンダリング
                this.clampedArray.set(sharedView);
                this.canvasCtx.putImageData(this.imageData, 0, 0);
            }

            this.dirtyRegion = null;
        } catch (error) {
            console.error("Failed to update canvas:", error);
            // エラー時はフルレンダリングにフォールバック
            try {
                const sharedView = new Uint8Array(this.sharedBuffer);
                this.clampedArray.set(sharedView);
                this.canvasCtx.putImageData(this.imageData, 0, 0);
            } catch (fallbackError) {
                console.error("Fallback rendering also failed:", fallbackError);
            }
        }
    }

    // SharedArrayBufferのビューを取得（再利用）
    private getSharedView(): Uint8Array {
        if (!this.sharedView || this.sharedView.buffer !== this.sharedBuffer) {
            this.sharedView = new Uint8Array(this.sharedBuffer!);
        }
        return this.sharedView;
    }

    // デバッグ用: DirtyRegionを視覚化
    private visualizeDirtyRegion(region: any): void {
        if (!this.canvasCtx) return;

        this.canvasCtx.save();
        this.canvasCtx.strokeStyle = "rgba(255, 0, 0, 0.3)";
        this.canvasCtx.lineWidth = 2;
        this.canvasCtx.strokeRect(region.x, region.y, region.width, region.height);
        this.canvasCtx.restore();

        // 1秒後に枠を消す
        setTimeout(() => {
            this.updateCanvas();
        }, 1000);
    }
}

// WASMキャンバスコンテキストを作成するファクトリ関数
export async function createWasmCanvasContext(
    canvas: HTMLCanvasElement,
    width: number,
    height: number
): Promise<DrawingContext> {
    const context = new WasmCanvasDrawingContext(canvas, width, height);
    await context.initialize();
    return context;
}
