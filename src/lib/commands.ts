/**
 * Tauri IPCコマンドの定義
 * @tauri-apps/api/coreを使用した型安全なコマンド呼び出し
 */

import { invoke } from '@tauri-apps/api/core';

// APIレスポンスの共通型
interface ApiResponse<T> {
    status: 'ok' | 'error';
    data?: T;
    error?: string;
}

// コマンド定義
export const commands = {
    // システム情報取得
    async getSystemInfo(): Promise<ApiResponse<string>> {
        try {
            const result = await invoke('get_system_info');
            return { status: 'ok', data: result as string };
        } catch (error) {
            return { 
                status: 'error', 
                error: error instanceof Error ? error.message : String(error) 
            };
        }
    },

    // 描画エンジン初期化
    async initializeDrawingEngine(): Promise<ApiResponse<string>> {
        try {
            const result = await invoke('initialize_drawing_engine');
            return { status: 'ok', data: result as string };
        } catch (error) {
            return { 
                status: 'error', 
                error: error instanceof Error ? error.message : String(error) 
            };
        }
    },

    // レイヤー作成
    async createDrawingLayer(layerId: string, width: number, height: number): Promise<ApiResponse<any>> {
        try {
            const result = await invoke('create_drawing_layer', { 
                layerId, 
                width, 
                height 
            });
            return { status: 'ok', data: result };
        } catch (error) {
            return { 
                status: 'error', 
                error: error instanceof Error ? error.message : String(error) 
            };
        }
    },

    // レイヤー削除
    async removeLayer(layerId: string): Promise<ApiResponse<any>> {
        try {
            const result = await invoke('remove_layer', { layerId });
            return { status: 'ok', data: result };
        } catch (error) {
            return { 
                status: 'error', 
                error: error instanceof Error ? error.message : String(error) 
            };
        }
    },

    // 描画状態取得
    async getDrawingState(): Promise<ApiResponse<any>> {
        try {
            const result = await invoke('get_drawing_state');
            return { status: 'ok', data: result };
        } catch (error) {
            return { 
                status: 'error', 
                error: error instanceof Error ? error.message : String(error) 
            };
        }
    },

    // ユーザー入力処理
    async processUserInput(input: any): Promise<ApiResponse<any>> {
        try {
            const result = await invoke('process_user_input', { input });
            return { status: 'ok', data: result };
        } catch (error) {
            return { 
                status: 'error', 
                error: error instanceof Error ? error.message : String(error) 
            };
        }
    }
};