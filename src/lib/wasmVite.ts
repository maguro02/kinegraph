import type { DrawingContext, Point, StrokeId, Color, BrushTool } from "../types/drawing.ts";

// Viteのvite-plugin-wasmを使用したインポート
// ここでは仮のパスを使用していますが、実際のWASMビルド成果物のパスに合わせて調整してください
import initWasm from "../../src-wasm/pkg/kinegraph_wasm_bg.wasm?init";

// WASMモジュールの型定義
interface WasmInstance {
    DrawingContext: {
        new (width: number, height: number): any;
    };
    SharedBuffer: {
        new (size: number): any;
    };
    check_webgpu_support: () => boolean;
    [key: string]: any;
}

// グローバルでWASMモジュールを管理
let wasmModule: WasmInstance | null = null;
let wasmInitPromise: Promise<void> | null = null;

// WASMモジュールの初期化
async function initializeWasmModule(): Promise<void> {
    if (wasmModule) return;
    if (wasmInitPromise) return wasmInitPromise;

    wasmInitPromise = (async () => {
        try {
            // vite-plugin-wasmを使用した初期化
            const wasm = await initWasm() as any;
            
            // WASMモジュールを保存
            wasmModule = wasm as WasmInstance;

            console.log("WASM module initialized successfully with vite-plugin-wasm");
        } catch (error) {
            console.error("Failed to initialize WASM module:", error);
            throw error;
        }
    })();

    return wasmInitPromise;
}

// ストロークデータを管理する型
interface StrokeData {
    points: Point[];
    tool: BrushTool;
    color: Color;
}

export class WasmViteDrawingContext implements DrawingContext {
    private drawingContext: any | null = null;
    private sharedBuffer: any | null = null;
    private ctx: CanvasRenderingContext2D | null = null;
    private imageData: ImageData | null = null;
    private activeStrokes: Map<StrokeId, StrokeData> = new Map();
    private strokeIdCounter = 0;
    private animationFrameId: number | null = null;
    private needsRedraw = false;

    constructor(private canvas: HTMLCanvasElement, private width: number, private height: number) {}

    async initialize(): Promise<void> {
        // WASMモジュールを初期化
        await initializeWasmModule();

        if (!wasmModule) {
            throw new Error("WASM module not available");
        }

        // Canvas 2Dコンテキストを取得（描画用）
        const ctx = this.canvas.getContext("2d", {
            alpha: false,
            desynchronized: true, // パフォーマンス向上のため
        });

        if (!ctx) {
            throw new Error("Failed to get 2D context");
        }

        this.ctx = ctx;
        this.canvas.width = this.width;
        this.canvas.height = this.height;

        // ImageDataを作成
        this.imageData = ctx.createImageData(this.width, this.height);

        // WebGPUサポートをチェック
        const hasWebGPU = wasmModule.check_webgpu_support();
        console.log("WebGPU support:", hasWebGPU);

        // DrawingContextを作成
        this.drawingContext = new wasmModule.DrawingContext(this.width, this.height);

        // SharedArrayBufferが利用可能な場合は、SharedBufferを作成
        if (typeof SharedArrayBuffer !== "undefined") {
            try {
                this.sharedBuffer = new wasmModule.SharedBuffer(this.width * this.height * 4);
                this.drawingContext.set_shared_buffer(this.sharedBuffer);
                console.log("SharedArrayBuffer enabled for high-performance rendering");
            } catch (e) {
                console.warn("Failed to create SharedBuffer:", e);
            }
        }

        // 初期クリア（白背景）
        this.drawingContext.clear(new Float32Array([1.0, 1.0, 1.0, 1.0]));
        this.updateCanvas();
    }

    async startStroke(point: Point, tool: BrushTool, color: Color): Promise<StrokeId> {
        if (!this.drawingContext) {
            throw new Error("Drawing context not initialized");
        }

        // 新しいストロークIDを生成
        const strokeId = (this.strokeIdCounter++).toString();

        // ストロークデータを初期化
        const strokeData: StrokeData = {
            points: [point],
            tool,
            color,
        };

        this.activeStrokes.set(strokeId, strokeData);

        // 最初の点を描画
        this.drawSinglePoint(point, tool, color);
        this.scheduleRedraw();

        return strokeId;
    }

    async addPoint(strokeId: StrokeId, point: Point): Promise<void> {
        if (!this.drawingContext) {
            throw new Error("Drawing context not initialized");
        }

        const strokeData = this.activeStrokes.get(strokeId);
        if (!strokeData) {
            throw new Error(`Stroke ${strokeId} not found`);
        }

        // 前回の点を取得
        const lastPoint = strokeData.points[strokeData.points.length - 1];
        if (lastPoint) {
            // 前回の点から今回の点までの線分を描画
            this.drawLine(lastPoint, point, strokeData.tool, strokeData.color);
        }

        // 新しい点を追加
        strokeData.points.push(point);
        this.scheduleRedraw();
    }

    async endStroke(strokeId: StrokeId): Promise<void> {
        if (!this.drawingContext) {
            throw new Error("Drawing context not initialized");
        }

        const strokeData = this.activeStrokes.get(strokeId);
        if (strokeData && strokeData.points.length >= 2) {
            // ストローク全体を一度に描画（スムージング処理などが可能）
            const points: number[] = [];
            strokeData.points.forEach((p) => {
                points.push(p.x, p.y);
            });

            const pointsArray = new Float32Array(points);
            const color = new Float32Array([
                strokeData.color.r / 255,
                strokeData.color.g / 255,
                strokeData.color.b / 255,
                strokeData.color.a,
            ]);

            this.drawingContext.draw_stroke(pointsArray, color, strokeData.tool.size);
        }

        this.activeStrokes.delete(strokeId);
        this.scheduleRedraw();
    }

    async clear(): Promise<void> {
        if (!this.drawingContext) {
            throw new Error("Drawing context not initialized");
        }

        // 白でクリア
        this.drawingContext.clear(new Float32Array([1.0, 1.0, 1.0, 1.0]));
        this.activeStrokes.clear();
        this.scheduleRedraw();
    }

    resize(width: number, height: number): void {
        this.width = width;
        this.height = height;

        if (this.canvas && this.ctx) {
            this.canvas.width = width;
            this.canvas.height = height;
            this.imageData = this.ctx.createImageData(width, height);
        }

        // WASMコンテキストも再作成が必要
        console.warn("Canvas resize not fully implemented for WASM context");
    }

    destroy(): void {
        if (this.animationFrameId !== null) {
            cancelAnimationFrame(this.animationFrameId);
        }

        if (this.drawingContext) {
            this.drawingContext.free();
        }

        if (this.sharedBuffer) {
            this.sharedBuffer.free();
        }

        this.drawingContext = null;
        this.sharedBuffer = null;
        this.ctx = null;
        this.imageData = null;
        this.activeStrokes.clear();
    }

    // 描画をスケジュール（パフォーマンス向上のため）
    private scheduleRedraw(): void {
        if (this.needsRedraw) return;

        this.needsRedraw = true;
        this.animationFrameId = requestAnimationFrame(() => {
            this.updateCanvas();
            this.needsRedraw = false;
            this.animationFrameId = null;
        });
    }

    // キャンバスを更新
    private updateCanvas(): void {
        if (!this.drawingContext || !this.ctx || !this.imageData) {
            return;
        }

        try {
            // SharedBufferが利用可能な場合
            if (this.sharedBuffer) {
                this.drawingContext.copy_to_shared_buffer();
                const buffer = new Uint8Array(this.sharedBuffer.buffer);
                this.imageData.data.set(buffer);
            } else {
                // SharedBufferが利用できない場合は、通常の方法でピクセルを取得
                const pixels = this.drawingContext.get_pixels();
                this.imageData.data.set(pixels);
            }

            // キャンバスに描画
            this.ctx.putImageData(this.imageData, 0, 0);
        } catch (error) {
            console.error("Failed to update canvas:", error);
        }
    }

    // 単一の点を描画
    private drawSinglePoint(point: Point, tool: BrushTool, color: Color): void {
        if (!this.drawingContext) return;

        const points = new Float32Array([point.x, point.y, point.x + 0.1, point.y + 0.1]);
        const colorArray = new Float32Array([color.r / 255, color.g / 255, color.b / 255, color.a]);

        this.drawingContext.draw_stroke(points, colorArray, tool.size);
    }

    // 2点間の線を描画
    private drawLine(start: Point, end: Point, tool: BrushTool, color: Color): void {
        if (!this.drawingContext) return;

        // 線分を複数の点で補間（スムーズな線のため）
        const distance = Math.sqrt((end.x - start.x) ** 2 + (end.y - start.y) ** 2);
        const steps = Math.max(2, Math.ceil(distance / 2)); // 2ピクセルごとに1点

        const points: number[] = [];
        for (let i = 0; i <= steps; i++) {
            const t = i / steps;
            points.push(start.x + (end.x - start.x) * t, start.y + (end.y - start.y) * t);
        }

        const pointsArray = new Float32Array(points);
        const colorArray = new Float32Array([color.r / 255, color.g / 255, color.b / 255, color.a]);

        this.drawingContext.draw_stroke(pointsArray, colorArray, tool.size);
    }
}

// WASMコンテキストを作成するファクトリ関数
export async function createWasmViteContext(
    canvas: HTMLCanvasElement,
    width: number,
    height: number
): Promise<DrawingContext> {
    const context = new WasmViteDrawingContext(canvas, width, height);
    await context.initialize();
    return context;
}