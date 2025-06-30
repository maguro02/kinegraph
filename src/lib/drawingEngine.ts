import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

// Rust側と同じ型定義
export interface DrawingUpdate {
  layerId: string;
  updateType: 'strokeProgress' | 'strokeComplete' | 'partialUpdate';
  data: number[];
  rect: UpdateRect;
}

export interface UpdateRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface StrokePoint {
  x: number;
  y: number;
  pressure: number;
}

/**
 * 描画エンジンクライアント
 * Rustの描画エンジンとリアルタイム通信を行う
 */
export class DrawingEngineClient {
  private updateListeners: Map<string, ((update: DrawingUpdate) => void)[]> = new Map();
  private strokeListeners: Map<string, ((strokeId: string) => void)[]> = new Map();
  private activeStrokes: Map<string, { layerId: string; startTime: number }> = new Map();
  private unlistenFns: (() => void)[] = [];

  constructor() {
    this.startListening();
  }

  /**
   * イベントリスニングを開始
   */
  private async startListening() {
    // 描画更新イベントをリッスン
    const unlistenUpdate = await listen<DrawingUpdate>('drawing-update', (event) => {
      this.handleDrawingUpdate(event.payload);
    });
    this.unlistenFns.push(unlistenUpdate);

    // ストローク開始イベントをリッスン
    const unlistenStart = await listen<string>('stroke-started', (event) => {
      this.handleStrokeStarted(event.payload);
    });
    this.unlistenFns.push(unlistenStart);

    // ストローク完了イベントをリッスン
    const unlistenComplete = await listen<string>('stroke-completed', (event) => {
      this.handleStrokeCompleted(event.payload);
    });
    this.unlistenFns.push(unlistenComplete);
  }

  /**
   * リスナーをクリーンアップ
   */
  public cleanup() {
    this.unlistenFns.forEach(fn => fn());
    this.unlistenFns = [];
    this.updateListeners.clear();
    this.strokeListeners.clear();
    this.activeStrokes.clear();
  }

  /**
   * 描画更新イベントハンドラー
   */
  private handleDrawingUpdate(update: DrawingUpdate) {
    const listeners = this.updateListeners.get(update.layerId) || [];
    listeners.forEach(listener => listener(update));
  }

  /**
   * ストローク開始イベントハンドラー
   */
  private handleStrokeStarted(strokeId: string) {
    console.debug(`[DrawingEngine] ストローク開始: ${strokeId}`);
  }

  /**
   * ストローク完了イベントハンドラー
   */
  private handleStrokeCompleted(strokeId: string) {
    console.debug(`[DrawingEngine] ストローク完了: ${strokeId}`);
    this.activeStrokes.delete(strokeId);
  }

  /**
   * レイヤー更新リスナーを登録
   */
  public addUpdateListener(layerId: string, callback: (update: DrawingUpdate) => void) {
    if (!this.updateListeners.has(layerId)) {
      this.updateListeners.set(layerId, []);
    }
    this.updateListeners.get(layerId)!.push(callback);
  }

  /**
   * レイヤー更新リスナーを削除
   */
  public removeUpdateListener(layerId: string, callback: (update: DrawingUpdate) => void) {
    const listeners = this.updateListeners.get(layerId);
    if (listeners) {
      const index = listeners.indexOf(callback);
      if (index !== -1) {
        listeners.splice(index, 1);
      }
    }
  }

  /**
   * リアルタイムストロークを開始
   */
  public async beginRealtimeStroke(
    layerId: string,
    color: [number, number, number, number],
    brushSize: number,
    tool: string
  ): Promise<string> {
    const strokeId = await invoke<string>('begin_realtime_stroke', {
      layerId,
      color,
      brushSize,
      tool
    });
    
    this.activeStrokes.set(strokeId, {
      layerId,
      startTime: Date.now()
    });
    
    return strokeId;
  }

  /**
   * リアルタイムストロークに点を追加
   */
  public async addRealtimeStrokePoint(
    strokeId: string,
    point: StrokePoint
  ): Promise<void> {
    if (!this.activeStrokes.has(strokeId)) {
      throw new Error(`アクティブなストロークが見つかりません: ${strokeId}`);
    }
    
    await invoke('add_realtime_stroke_point', {
      strokeId,
      point
    });
  }

  /**
   * リアルタイムストロークを完了
   */
  public async completeRealtimeStroke(strokeId: string): Promise<void> {
    if (!this.activeStrokes.has(strokeId)) {
      throw new Error(`アクティブなストロークが見つかりません: ${strokeId}`);
    }
    
    await invoke('complete_realtime_stroke', {
      strokeId
    });
    
    this.activeStrokes.delete(strokeId);
  }

  /**
   * キャンバスに部分更新を適用
   */
  public applyPartialUpdate(
    canvas: HTMLCanvasElement,
    update: DrawingUpdate
  ) {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Uint8ClampedArrayに変換
    const data = new Uint8ClampedArray(update.data);
    
    // 部分的な画像データを作成
    const imageData = new ImageData(
      data,
      update.rect.width,
      update.rect.height
    );
    
    // 指定された領域に描画
    ctx.putImageData(imageData, update.rect.x, update.rect.y);
  }
}

// シングルトンインスタンス
export const drawingEngine = new DrawingEngineClient();