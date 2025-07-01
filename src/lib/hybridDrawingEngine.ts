import { invoke } from "@tauri-apps/api/core";
import type { UserInput, DrawCommand, DrawingStateInfo } from "./hybridCommands";

/**
 * ハイブリッド描画エンジンAPIクライアント
 */
export class HybridDrawingEngine {
  /**
   * ユーザー入力を処理し、描画コマンドを受け取る
   */
  static async processUserInput(input: UserInput): Promise<DrawCommand[]> {
    try {
      const commands = await invoke<DrawCommand[]>("process_user_input", {
        input,
      });
      return commands;
    } catch (error) {
      console.error("Failed to process user input:", error);
      throw error;
    }
  }

  /**
   * 現在の描画状態を取得
   */
  static async getDrawingState(): Promise<DrawingStateInfo> {
    try {
      const state = await invoke<DrawingStateInfo>("get_drawing_state");
      return state;
    } catch (error) {
      console.error("Failed to get drawing state:", error);
      throw error;
    }
  }

  /**
   * ストロークを描画（簡易ヘルパー）
   */
  static async drawStroke(
    points: { x: number; y: number }[],
    color: string,
    width: number,
    layerId: string
  ): Promise<DrawCommand[]> {
    return this.processUserInput({
      type: "DrawStroke",
      payload: {
        points,
        color,
        width,
        layer_id: layerId,
      },
    });
  }

  /**
   * レイヤーを作成（簡易ヘルパー）
   */
  static async createLayer(name: string): Promise<DrawCommand[]> {
    return this.processUserInput({
      type: "CreateLayer",
      payload: { name },
    });
  }

  /**
   * ツールを変更（簡易ヘルパー）
   */
  static async changeTool(toolId: string): Promise<DrawCommand[]> {
    return this.processUserInput({
      type: "ChangeTool",
      payload: { tool_id: toolId },
    });
  }

  /**
   * Undo実行
   */
  static async undo(): Promise<DrawCommand[]> {
    return this.processUserInput({
      type: "Undo",
      payload: {},
    });
  }

  /**
   * Redo実行
   */
  static async redo(): Promise<DrawCommand[]> {
    return this.processUserInput({
      type: "Redo",
      payload: {},
    });
  }
}