import { invoke } from "@tauri-apps/api/core";

/**
 * フロントエンド側のログ出力関数
 */
function logToConsole(level: "info" | "error" | "debug" | "warn", message: string, data?: any) {
    const timestamp = new Date().toISOString();
    const logMessage = `[${timestamp}] [KINEGRAPH-DRAWING] ${message}`;

    switch (level) {
        case "info":
            console.info(logMessage, data || "");
            break;
        case "error":
            console.error(logMessage, data || "");
            break;
        case "debug":
            console.debug(logMessage, data || "");
            break;
        case "warn":
            console.warn(logMessage, data || "");
            break;
    }
}

/**
 * TypeScript型定義
 */
export interface StrokePoint {
    x: number;
    y: number;
    pressure: number;
}

export interface DrawingLayer {
    id: string;
    width: number;
    height: number;
}

export interface DrawingStats {
    layers_count: number;
    memory_used: number;
    memory_limit: number;
    active_textures: number;
    total_textures: number;
}

export interface Color {
    r: number;
    g: number;
    b: number;
    a: number;
}

/**
 * DrawingEngineクラス - Rustドローエンジンとの統合API
 */
export class DrawingEngine {
    private isInitialized = false;
    private layers = new Map<string, DrawingLayer>();

    /**
     * 描画エンジンを初期化
     */
    async initialize(): Promise<void> {
        // 重複初期化チェック
        if (this.isInitialized) {
            logToConsole("warn", "描画エンジンは既に初期化されています。スキップします。");
            return;
        }

        logToConsole("info", "描画エンジンの初期化を開始", {
            currentState: this.isInitialized,
            layersCount: this.layers.size,
            existingLayers: Array.from(this.layers.keys())
        });

        try {
            const invokeStartTime = performance.now();
            const result = await invoke<string>("initialize_drawing_engine");
            const invokeEndTime = performance.now();
            
            this.isInitialized = true;
            
            logToConsole("info", "描画エンジン初期化完了", {
                result,
                executionTime: `${(invokeEndTime - invokeStartTime).toFixed(2)}ms`,
                newState: this.isInitialized
            });
        } catch (error) {
            logToConsole("error", "描画エンジン初期化でエラーが発生", {
                error,
                errorType: typeof error,
                errorString: String(error),
                isInitialized: this.isInitialized
            });
            this.handleError("描画エンジンの初期化に失敗しました", error);
        }
    }

    /**
     * レイヤーを作成
     */
    async createLayer(id: string, width: number, height: number): Promise<string> {
        logToConsole("info", `レイヤー作成開始: ${id} (${width}x${height})`, {
            layerId: id,
            width,
            height,
            engineInitialized: this.isInitialized,
            existingLayersCount: this.layers.size,
            existingLayerIds: Array.from(this.layers.keys()),
            layerAlreadyExists: this.layers.has(id)
        });

        // 引数検証
        if (!id || typeof id !== 'string') {
            const error = new Error(`無効なレイヤーID: ${id}`);
            logToConsole("error", "レイヤー作成: 無効な引数", { id, idType: typeof id });
            throw error;
        }

        if (width <= 0 || height <= 0 || !Number.isInteger(width) || !Number.isInteger(height)) {
            const error = new Error(`無効なサイズ: ${width}x${height}`);
            logToConsole("error", "レイヤー作成: 無効なサイズ", { width, height, widthType: typeof width, heightType: typeof height });
            throw error;
        }

        this.checkInitialized();

        // 既存レイヤーチェック
        if (this.layers.has(id)) {
            logToConsole("warn", `レイヤーID ${id} は既に存在します。上書きします。`);
        }

        try {
            const invokeStartTime = performance.now();
            logToConsole("debug", `Tauri invoke呼び出し前: create_drawing_layer`, {
                parameters: { layerId: id, width, height },
                timestamp: new Date().toISOString()
            });

            const result = await invoke<string>("create_drawing_layer", {
                layerId: id,
                width,
                height,
            });

            const invokeEndTime = performance.now();
            logToConsole("debug", `Tauri invoke呼び出し後: create_drawing_layer`, {
                result,
                executionTime: `${(invokeEndTime - invokeStartTime).toFixed(2)}ms`,
                timestamp: new Date().toISOString()
            });

            // ローカル状態も更新
            this.layers.set(id, { id, width, height });
            
            logToConsole("info", `レイヤー作成完了: ${id}`, {
                result,
                newLayersCount: this.layers.size,
                allLayerIds: Array.from(this.layers.keys()),
                executionTime: `${(invokeEndTime - invokeStartTime).toFixed(2)}ms`
            });
            
            return result;
        } catch (error) {
            logToConsole("error", `レイヤー作成でエラーが発生: ${id}`, {
                error,
                errorType: typeof error,
                errorMessage: error instanceof Error ? error.message : String(error),
                errorStack: error instanceof Error ? error.stack : undefined,
                inputParameters: { layerId: id, width, height },
                engineState: {
                    isInitialized: this.isInitialized,
                    layersCount: this.layers.size,
                    layerIds: Array.from(this.layers.keys())
                }
            });
            this.handleError(`レイヤーの作成に失敗しました: ${id}`, error);
        }
    }

    /**
     * レイヤーに線を描画
     */
    async drawLine(
        layerId: string,
        x1: number,
        y1: number,
        x2: number,
        y2: number,
        color: Color,
        width: number = 2.0
    ): Promise<void> {
        logToConsole("debug", `線描画: ${layerId} (${x1},${y1}) -> (${x2},${y2})`);
        this.checkInitialized();
        this.checkLayerExists(layerId);

        try {
            await invoke("draw_line_on_layer", {
                layerId,
                x1,
                y1,
                x2,
                y2,
                color: [color.r, color.g, color.b, color.a],
                width,
            });

            logToConsole("debug", `線描画完了: ${layerId}`);
        } catch (error) {
            logToConsole("error", `線描画でエラーが発生: ${layerId}`, error);
            this.handleError(`線の描画に失敗しました: ${layerId}`, error);
        }
    }

    /**
     * レイヤーにストロークを描画（筆圧対応）
     */
    async drawStroke(layerId: string, points: StrokePoint[], color: Color): Promise<void> {
        logToConsole("debug", `ストローク描画: ${layerId} (${points.length}点)`);
        this.checkInitialized();
        this.checkLayerExists(layerId);

        if (points.length === 0) {
            throw new Error("ストロークの点が空です。最低1つの点が必要です。");
        }

        try {
            await invoke("draw_stroke_on_layer", {
                layerId,
                points,
                color: [color.r, color.g, color.b, color.a],
            });

            logToConsole("debug", `ストローク描画完了: ${layerId}`);
        } catch (error) {
            logToConsole("error", `ストローク描画でエラーが発生: ${layerId}`, error);
            this.handleError(`ストロークの描画に失敗しました: ${layerId}`, error);
        }
    }

    /**
     * レイヤーの画像データを取得
     */
    async getLayerImageData(layerId: string): Promise<Uint8Array> {
        logToConsole("debug", `レイヤー画像データ取得: ${layerId}`);
        this.checkInitialized();
        this.checkLayerExists(layerId);

        try {
            const data = await invoke<number[]>("get_layer_image_data", {
                layerId,
            });

            const imageData = new Uint8Array(data);
            logToConsole("debug", `レイヤー画像データ取得完了: ${layerId} (${imageData.length}バイト)`);
            return imageData;
        } catch (error) {
            logToConsole("error", `レイヤー画像データ取得でエラーが発生: ${layerId}`, error);
            this.handleError(`レイヤーの画像データ取得に失敗しました: ${layerId}`, error);
        }
    }

    /**
     * Canvasにレイヤーを描画
     */
    async renderToCanvas(canvas: HTMLCanvasElement, layerId: string): Promise<void> {
        logToConsole("debug", `Canvas描画: ${layerId}`);
        this.checkInitialized();
        this.checkLayerExists(layerId);

        try {
            const layer = this.layers.get(layerId)!;
            const imageData = await this.getLayerImageData(layerId);

            // Canvasコンテキストを取得
            const ctx = canvas.getContext("2d");
            if (!ctx) {
                throw new Error("Canvasコンテキストの取得に失敗しました");
            }

            // Canvas サイズをレイヤーサイズに合わせる
            canvas.width = layer.width;
            canvas.height = layer.height;

            // ImageDataオブジェクトを作成
            const canvasImageData = ctx.createImageData(layer.width, layer.height);
            canvasImageData.data.set(imageData);

            // Canvasに描画
            ctx.putImageData(canvasImageData, 0, 0);
            
            logToConsole("debug", `Canvas描画完了: ${layerId}`);
        } catch (error) {
            logToConsole("error", `Canvas描画でエラーが発生: ${layerId}`, error);
            this.handleError(`Canvasへの描画に失敗しました: ${layerId}`, error);
        }
    }

    /**
     * レイヤーをクリア
     */
    async clearLayer(layerId: string): Promise<void> {
        logToConsole("info", `レイヤークリア: ${layerId}`);
        this.checkInitialized();
        this.checkLayerExists(layerId);

        try {
            await invoke("clear_layer", {
                layerId,
            });

            logToConsole("info", `レイヤークリア完了: ${layerId}`);
        } catch (error) {
            logToConsole("error", `レイヤークリアでエラーが発生: ${layerId}`, error);
            this.handleError(`レイヤーのクリアに失敗しました: ${layerId}`, error);
        }
    }

    /**
     * レイヤーを削除
     */
    async removeLayer(layerId: string): Promise<void> {
        logToConsole("info", `レイヤー削除: ${layerId}`);
        this.checkInitialized();
        this.checkLayerExists(layerId);

        try {
            await invoke("remove_layer", {
                layerId,
            });

            // ローカル状態からも削除
            this.layers.delete(layerId);
            
            logToConsole("info", `レイヤー削除完了: ${layerId}`);
        } catch (error) {
            logToConsole("error", `レイヤー削除でエラーが発生: ${layerId}`, error);
            this.handleError(`レイヤーの削除に失敗しました: ${layerId}`, error);
        }
    }

    /**
     * 描画エンジンの統計情報を取得
     */
    async getStats(): Promise<DrawingStats> {
        logToConsole("debug", "描画エンジン統計情報取得");
        this.checkInitialized();

        try {
            const stats = await invoke<DrawingStats>("get_drawing_stats");
            logToConsole("debug", "描画エンジン統計情報取得完了", stats);
            return stats;
        } catch (error) {
            logToConsole("error", "描画エンジン統計情報取得でエラーが発生", error);
            this.handleError("描画エンジンの統計情報取得に失敗しました", error);
        }
    }

    /**
     * 未使用のテクスチャをクリーンアップ
     */
    async cleanupTextures(): Promise<void> {
        logToConsole("info", "テクスチャクリーンアップ開始");
        this.checkInitialized();

        try {
            const result = await invoke<string>("cleanup_textures");
            logToConsole("info", "テクスチャクリーンアップ完了", result);
        } catch (error) {
            logToConsole("error", "テクスチャクリーンアップでエラーが発生", error);
            this.handleError("テクスチャのクリーンアップに失敗しました", error);
        }
    }

    /**
     * 作成済みレイヤー一覧を取得
     */
    getLayerList(): DrawingLayer[] {
        return Array.from(this.layers.values());
    }

    /**
     * レイヤー情報を取得
     */
    getLayer(layerId: string): DrawingLayer | undefined {
        return this.layers.get(layerId);
    }

    /**
     * 初期化状態をチェック
     */
    private checkInitialized(): void {
        if (!this.isInitialized) {
            throw new Error("描画エンジンが初期化されていません。initialize()を先に実行してください。");
        }
    }

    /**
     * レイヤーの存在をチェック
     */
    private checkLayerExists(layerId: string): void {
        if (!this.layers.has(layerId)) {
            throw new Error(`レイヤーが見つかりません: ${layerId}`);
        }
    }

    /**
     * エラーハンドリング
     */
    private handleError(message: string, error: unknown): never {
        let errorMessage = message;
        let errorDetails: any = {};

        if (error instanceof Error) {
            errorMessage += `: ${error.message}`;
            errorDetails = {
                name: error.name,
                message: error.message,
                stack: error.stack,
                cause: (error as any).cause
            };
            logToConsole("error", "エラー詳細情報", errorDetails);
        } else if (typeof error === "string") {
            errorMessage += `: ${error}`;
            errorDetails = { stringError: error };
        } else if (error && typeof error === "object") {
            // Rustからのエラーオブジェクトの場合
            errorDetails = {
                errorObject: error,
                errorKeys: Object.keys(error),
                errorString: JSON.stringify(error, null, 2)
            };
            
            // 特定のRustエラーフォーマットをチェック
            if ('message' in error) {
                errorMessage += `: ${(error as any).message}`;
            } else {
                errorMessage += `: ${JSON.stringify(error)}`;
            }
            
            logToConsole("error", "Rustエラーオブジェクト詳細", errorDetails);
        } else {
            errorMessage += `: ${JSON.stringify(error)}`;
            errorDetails = { unknownError: error, errorType: typeof error };
        }

        // システム状態も記録
        logToConsole("error", "システム状態（エラー発生時）", {
            engineInitialized: this.isInitialized,
            layersCount: this.layers.size,
            layerIds: Array.from(this.layers.keys()),
            timestamp: new Date().toISOString(),
            userAgent: navigator.userAgent,
            url: window.location.href
        });

        throw new Error(errorMessage);
    }
}

/**
 * ユーティリティ関数
 */

/**
 * RGB値からColorオブジェクトを作成
 */
export function createColor(r: number, g: number, b: number, a: number = 1.0): Color {
    return {
        r: Math.max(0, Math.min(1, r)),
        g: Math.max(0, Math.min(1, g)),
        b: Math.max(0, Math.min(1, b)),
        a: Math.max(0, Math.min(1, a)),
    };
}

/**
 * HEXカラーコードからColorオブジェクトを作成
 */
export function createColorFromHex(hex: string, alpha: number = 1.0): Color {
    const cleanHex = hex.replace('#', '');
    const r = parseInt(cleanHex.substr(0, 2), 16) / 255;
    const g = parseInt(cleanHex.substr(2, 2), 16) / 255;
    const b = parseInt(cleanHex.substr(4, 2), 16) / 255;
    
    return createColor(r, g, b, alpha);
}

/**
 * グローバルDrawingEngineインスタンス（シングルトン）
 */
export const drawingEngine = new DrawingEngine();