// 型安全なバインディングをインポート
import { commands, type Project as GeneratedProject } from "./bindings";

// atoms.tsのProject型とbindings.tsのProject型を変換
function convertToAtomsProject(project: GeneratedProject): import("../store/atoms").Project {
    return {
        id: `project-${Date.now()}`, // IDを生成
        name: project.name,
        width: project.width,
        height: project.height,
        frameRate: project.frame_rate, // snake_caseからcamelCaseに変換
        frames: project.frames.map(frame => ({
            ...frame,
            layers: frame.layers.map(layer => ({
                ...layer,
                blendMode: layer.blend_mode as any, // snake_caseからcamelCaseに変換
            }))
        }))
    };
}

/**
 * フロントエンド側のログ出力関数
 */
function logToConsole(level: "info" | "error" | "debug" | "warn", message: string, data?: any) {
    const timestamp = new Date().toISOString();
    const logMessage = `[${timestamp}] [KINEGRAPH-FRONTEND] ${message}`;

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

export async function createProject(name: string, width: number, height: number, frameRate: number): Promise<import("../store/atoms").Project> {
    logToConsole("info", "createProject 関数呼び出し開始");
    logToConsole("debug", "プロジェクトパラメータ", { name, width, height, frameRate });

    try {
        logToConsole("debug", "Tauri invoke create_project 実行中...");

        const result = await commands.createProject({
            name,
            width,
            height,
            frame_rate: frameRate,
        });

        if (result.status === "ok") {
            logToConsole("info", "create_project コマンド正常完了");
            logToConsole("debug", "プロジェクト作成結果", result.data);
            return convertToAtomsProject(result.data);
        } else {
            throw new Error(result.error);
        }
    } catch (error) {
        logToConsole("error", "create_project コマンドでエラーが発生", error);

        // エラーの詳細を分析
        if (error instanceof Error) {
            logToConsole("error", "エラーメッセージ", error.message);
            logToConsole("error", "エラースタック", error.stack);
        } else if (typeof error === "string") {
            logToConsole("error", "エラー文字列", error);
        } else {
            logToConsole("error", "未知のエラー形式", JSON.stringify(error));
        }

        throw error;
    }
}

export async function getSystemInfo(): Promise<string> {
    logToConsole("info", "getSystemInfo 関数呼び出し開始");

    try {
        logToConsole("debug", "Tauri invoke get_system_info 実行中...");

        const result = await commands.getSystemInfo();

        if (result.status === "ok") {
            logToConsole("info", "get_system_info コマンド正常完了");
            logToConsole("debug", "システム情報取得結果", result.data);
            return result.data;
        } else {
            throw new Error(result.error);
        }
    } catch (error) {
        logToConsole("error", "get_system_info コマンドでエラーが発生", error);

        // エラーの詳細を分析
        if (error instanceof Error) {
            logToConsole("error", "エラーメッセージ", error.message);
            logToConsole("error", "エラースタック", error.stack);
        } else if (typeof error === "string") {
            logToConsole("error", "エラー文字列", error);
        } else {
            logToConsole("error", "未知のエラー形式", JSON.stringify(error));
        }

        throw error;
    }
}

/**
 * 描画エンジンを初期化
 */
export async function initializeDrawingEngine(): Promise<string> {
    logToConsole("info", "initializeDrawingEngine 関数呼び出し開始");

    try {
        logToConsole("debug", "Tauri invoke initialize_drawing_engine 実行中...");

        const result = await commands.initializeDrawingEngine();

        if (result.status === "ok") {
            logToConsole("info", "initialize_drawing_engine コマンド正常完了");
            logToConsole("debug", "描画エンジン初期化結果", result.data);
            return result.data;
        } else {
            throw new Error(result.error);
        }
    } catch (error) {
        logToConsole("error", "initialize_drawing_engine コマンドでエラーが発生", error);
        throw error;
    }
}

/**
 * 描画レイヤーを作成
 */
export async function createDrawingLayer(layerId: string, width: number, height: number): Promise<string> {
    logToConsole("info", "createDrawingLayer 関数呼び出し開始");
    logToConsole("debug", "レイヤーパラメータ", { layerId, width, height });

    try {
        logToConsole("debug", "Tauri invoke create_drawing_layer 実行中...");

        const result = await commands.createDrawingLayer(layerId, width, height);

        if (result.status === "ok") {
            logToConsole("info", "create_drawing_layer コマンド正常完了");
            logToConsole("debug", "レイヤー作成結果", result.data);
            return result.data;
        } else {
            throw new Error(result.error);
        }
    } catch (error) {
        logToConsole("error", "create_drawing_layer コマンドでエラーが発生", error);
        throw error;
    }
}

/**
 * レイヤーに線を描画
 */
export async function drawLineOnLayer(
    layerId: string,
    x1: number,
    y1: number,
    x2: number,
    y2: number,
    color: [number, number, number, number],
    width: number
): Promise<void> {
    logToConsole("debug", "drawLineOnLayer 関数呼び出し開始");

    try {
        const result = await commands.drawLineOnLayer(layerId, x1, y1, x2, y2, color, width);

        if (result.status === "ok") {
            logToConsole("debug", "draw_line_on_layer コマンド正常完了");
        } else {
            throw new Error(result.error);
        }
    } catch (error) {
        logToConsole("error", "draw_line_on_layer コマンドでエラーが発生", error);
        throw error;
    }
}

/**
 * レイヤーにストロークを描画
 */
export interface StrokePoint {
    x: number;
    y: number;
    pressure: number;
}

export async function drawStrokeOnLayer(
    layerId: string,
    points: StrokePoint[],
    color: [number, number, number, number]
): Promise<void> {
    logToConsole("debug", "drawStrokeOnLayer 関数呼び出し開始");

    try {
        const result = await commands.drawStrokeOnLayer(layerId, points, color);

        if (result.status === "ok") {
            logToConsole("debug", "draw_stroke_on_layer コマンド正常完了");
        } else {
            throw new Error(result.error);
        }
    } catch (error) {
        logToConsole("error", "draw_stroke_on_layer コマンドでエラーが発生", error);
        throw error;
    }
}

/**
 * レイヤーの画像データを取得
 */
export async function getLayerImageData(layerId: string): Promise<number[]> {
    logToConsole("debug", "getLayerImageData 関数呼び出し開始");

    try {
        const result = await commands.getLayerImageData(layerId);

        if (result.status === "ok") {
            logToConsole("debug", "get_layer_image_data コマンド正常完了");
            return result.data;
        } else {
            throw new Error(result.error);
        }
    } catch (error) {
        logToConsole("error", "get_layer_image_data コマンドでエラーが発生", error);
        throw error;
    }
}

/**
 * レイヤーをクリア
 */
export async function clearLayer(layerId: string): Promise<void> {
    logToConsole("debug", "clearLayer 関数呼び出し開始");

    try {
        const result = await commands.clearLayer(layerId);

        if (result.status === "ok") {
            logToConsole("debug", "clear_layer コマンド正常完了");
        } else {
            throw new Error(result.error);
        }
    } catch (error) {
        logToConsole("error", "clear_layer コマンドでエラーが発生", error);
        throw error;
    }
}

/**
 * レイヤーを削除
 */
export async function removeLayer(layerId: string): Promise<void> {
    logToConsole("debug", "removeLayer 関数呼び出し開始");

    try {
        const result = await commands.removeLayer(layerId);

        if (result.status === "ok") {
            logToConsole("debug", "remove_layer コマンド正常完了");
        } else {
            throw new Error(result.error);
        }
    } catch (error) {
        logToConsole("error", "remove_layer コマンドでエラーが発生", error);
        throw error;
    }
}

/**
 * デバッグ用: アプリケーション起動時にシステム情報を取得してログ出力
 */
export async function initializeDebugLogging(): Promise<void> {
    logToConsole("info", "デバッグログ初期化開始");

    try {
        const systemInfo = await getSystemInfo();
        logToConsole("info", "システム初期化確認完了", systemInfo);
    } catch (error) {
        logToConsole("error", "システム初期化確認でエラー", error);
    }
}
