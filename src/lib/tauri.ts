import { invoke } from '@tauri-apps/api/core';
import type { Project } from '../store/atoms';

/**
 * フロントエンド側のログ出力関数
 */
function logToConsole(level: 'info' | 'error' | 'debug' | 'warn', message: string, data?: any) {
  const timestamp = new Date().toISOString();
  const logMessage = `[${timestamp}] [KINEGRAPH-FRONTEND] ${message}`;
  
  switch (level) {
    case 'info':
      console.info(logMessage, data || '');
      break;
    case 'error':
      console.error(logMessage, data || '');
      break;
    case 'debug':
      console.debug(logMessage, data || '');
      break;
    case 'warn':
      console.warn(logMessage, data || '');
      break;
  }
}

export async function createProject(
  name: string,
  width: number,
  height: number,
  frameRate: number
): Promise<Project> {
  logToConsole('info', 'createProject 関数呼び出し開始');
  logToConsole('debug', 'プロジェクトパラメータ', { name, width, height, frameRate });
  
  try {
    logToConsole('debug', 'Tauri invoke create_project 実行中...');
    
    const result = await invoke('create_project', {
      args: {
        name,
        width,
        height,
        frameRate
      }
    });
    
    logToConsole('info', 'create_project コマンド正常完了');
    logToConsole('debug', 'プロジェクト作成結果', result);
    
    return result as Project;
    
  } catch (error) {
    logToConsole('error', 'create_project コマンドでエラーが発生', error);
    
    // エラーの詳細を分析
    if (error instanceof Error) {
      logToConsole('error', 'エラーメッセージ', error.message);
      logToConsole('error', 'エラースタック', error.stack);
    } else if (typeof error === 'string') {
      logToConsole('error', 'エラー文字列', error);
    } else {
      logToConsole('error', '未知のエラー形式', JSON.stringify(error));
    }
    
    throw error;
  }
}

export async function getSystemInfo(): Promise<string> {
  logToConsole('info', 'getSystemInfo 関数呼び出し開始');
  
  try {
    logToConsole('debug', 'Tauri invoke get_system_info 実行中...');
    
    const result = await invoke('get_system_info');
    
    logToConsole('info', 'get_system_info コマンド正常完了');
    logToConsole('debug', 'システム情報取得結果', result);
    
    return result as string;
    
  } catch (error) {
    logToConsole('error', 'get_system_info コマンドでエラーが発生', error);
    
    // エラーの詳細を分析
    if (error instanceof Error) {
      logToConsole('error', 'エラーメッセージ', error.message);
      logToConsole('error', 'エラースタック', error.stack);
    } else if (typeof error === 'string') {
      logToConsole('error', 'エラー文字列', error);
    } else {
      logToConsole('error', '未知のエラー形式', JSON.stringify(error));
    }
    
    throw error;
  }
}

/**
 * デバッグ用: アプリケーション起動時にシステム情報を取得してログ出力
 */
export async function initializeDebugLogging(): Promise<void> {
  logToConsole('info', 'デバッグログ初期化開始');
  
  try {
    const systemInfo = await getSystemInfo();
    logToConsole('info', 'システム初期化確認完了', systemInfo);
  } catch (error) {
    logToConsole('error', 'システム初期化確認でエラー', error);
  }
}