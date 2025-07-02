// 型安全なバインディングをインポート
import { commands } from "./commands";

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

export async function getSystemInfo(): Promise<string> {
    logToConsole("info", "getSystemInfo 関数呼び出し開始");

    try {
        logToConsole("debug", "Tauri invoke get_system_info 実行中...");

        const result = await commands.getSystemInfo();

        if (result.status === "ok" && result.data) {
            logToConsole("info", "get_system_info コマンド正常完了");
            logToConsole("debug", "システム情報取得結果", result.data);
            return result.data;
        } else {
            throw new Error(result.error || "Unknown error");
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

        if (result.status === "ok" && result.data) {
            logToConsole("info", "initialize_drawing_engine コマンド正常完了");
            logToConsole("debug", "描画エンジン初期化結果", result.data);
            return result.data;
        } else {
            throw new Error(result.error || "Unknown error");
        }
    } catch (error) {
        logToConsole("error", "initialize_drawing_engine コマンドでエラーが発生", error);

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
 * 描画レイヤーを作成
 */
export async function createDrawingLayer(layerId: string, width: number, height: number) {
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
 * レイヤーを削除
 */
export async function removeLayer(layerId: string) {
    logToConsole("debug", "removeLayer 関数呼び出し開始");

    try {
        const result = await commands.removeLayer(layerId);

        if (result.status === "ok") {
            logToConsole("debug", "remove_layer コマンド正常完了");
            return result.data;
        } else {
            throw new Error(result.error);
        }
    } catch (error) {
        logToConsole("error", "remove_layer コマンドでエラーが発生", error);
        throw error;
    }
}

/**
 * 現在の描画状態を取得
 */
export async function getDrawingState() {
    logToConsole("debug", "getDrawingState 関数呼び出し開始");

    try {
        const result = await commands.getDrawingState();

        if (result.status === "ok") {
            logToConsole("debug", "get_drawing_state コマンド正常完了");
            return result.data;
        } else {
            throw new Error(result.error);
        }
    } catch (error) {
        logToConsole("error", "get_drawing_state コマンドでエラーが発生", error);
        throw error;
    }
}

/**
 * ユーザー入力を処理
 */
export async function processUserInput(input: any) {
    logToConsole("debug", "processUserInput 関数呼び出し開始");

    try {
        const result = await commands.processUserInput(input);

        if (result.status === "ok") {
            logToConsole("debug", "process_user_input コマンド正常完了");
            return result.data;
        } else {
            throw new Error(result.error);
        }
    } catch (error) {
        logToConsole("error", "process_user_input コマンドでエラーが発生", error);
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
