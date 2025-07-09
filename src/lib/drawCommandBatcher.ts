/**
 * 描画コマンドバッチャー
 * 複数の描画コマンドをバッチ処理して、パフォーマンスを最適化
 */

import { DrawEngineCommand } from './bindings';

export interface DrawCommandBatch {
  commands: DrawEngineCommand[];
  timestamp: number;
}

export interface DrawCommandBatcherOptions {
  maxBatchSize: number;        // バッチの最大コマンド数
  flushInterval: number;       // 自動フラッシュ間隔（ミリ秒）
  priorityThreshold: number;   // 優先度の閾値（これ以上は即座に送信）
}

export class DrawCommandBatcher {
  private queue: DrawEngineCommand[] = [];
  private flushTimer: number | null = null;
  private onFlush: (batch: DrawCommandBatch) => Promise<void>;
  private options: DrawCommandBatcherOptions;

  constructor(
    onFlush: (batch: DrawCommandBatch) => Promise<void>,
    options: Partial<DrawCommandBatcherOptions> = {}
  ) {
    this.onFlush = onFlush;
    this.options = {
      maxBatchSize: options.maxBatchSize ?? 50,
      flushInterval: options.flushInterval ?? 16, // 約60fps
      priorityThreshold: options.priorityThreshold ?? 10,
    };
  }

  /**
   * コマンドをキューに追加
   */
  add(command: DrawEngineCommand): void {
    this.queue.push(command);

    // 優先度の高いコマンドは即座にフラッシュ
    if (this.isPriorityCommand(command)) {
      this.flush();
      return;
    }

    // バッチサイズが上限に達したらフラッシュ
    if (this.queue.length >= this.options.maxBatchSize) {
      this.flush();
      return;
    }

    // タイマーが設定されていない場合は設定
    if (!this.flushTimer) {
      this.scheduleFlush();
    }
  }

  /**
   * 優先度の高いコマンドかどうかを判定
   */
  private isPriorityCommand(command: DrawEngineCommand): boolean {
    // endStrokeやレイヤー操作など、即座に反映すべきコマンド
    if (typeof command === 'string') {
      return command === 'endStroke' || command === 'clear' || command === 'createLayer';
    }
    if ('deleteLayer' in command || 'setActiveLayer' in command) {
      return true;
    }
    return false;
  }

  /**
   * バッチのフラッシュをスケジュール
   */
  private scheduleFlush(): void {
    this.flushTimer = window.setTimeout(() => {
      this.flush();
    }, this.options.flushInterval);
  }

  /**
   * キューをフラッシュして送信
   */
  async flush(): Promise<void> {
    if (this.queue.length === 0) {
      return;
    }

    // タイマーをクリア
    if (this.flushTimer) {
      clearTimeout(this.flushTimer);
      this.flushTimer = null;
    }

    // バッチを作成
    const batch: DrawCommandBatch = {
      commands: [...this.queue],
      timestamp: Date.now(),
    };

    // キューをクリア
    this.queue = [];

    // バッチを送信
    try {
      await this.onFlush(batch);
    } catch (error) {
      console.error('Failed to flush command batch:', error);
      // エラー時は個別にコマンドを再送信する可能性があるため、
      // キューに戻すことも検討
    }
  }

  /**
   * バッチャーを破棄
   */
  destroy(): void {
    if (this.flushTimer) {
      clearTimeout(this.flushTimer);
      this.flushTimer = null;
    }
    this.queue = [];
  }

  /**
   * 現在のキューサイズを取得
   */
  getQueueSize(): number {
    return this.queue.length;
  }

  /**
   * バッチ処理の統計情報
   */
  getStats(): {
    queueSize: number;
    hasPendingFlush: boolean;
  } {
    return {
      queueSize: this.queue.length,
      hasPendingFlush: this.flushTimer !== null,
    };
  }
}

/**
 * 描画コマンドの最適化
 * 連続する同じタイプのコマンドを結合して効率化
 */
export function optimizeCommands(commands: DrawEngineCommand[]): DrawEngineCommand[] {
  if (commands.length <= 1) {
    return commands;
  }

  const optimized: DrawEngineCommand[] = [];
  let i = 0;

  while (i < commands.length) {
    const current = commands[i];

    // continueStrokeコマンドの結合
    if (typeof current === 'object' && 'continueStroke' in current) {
      const points = [current.continueStroke];
      let j = i + 1;

      // 連続するcontinueStrokeを収集
      while (j < commands.length) {
        const next = commands[j];
        if (typeof next === 'object' && 'continueStroke' in next) {
          points.push(next.continueStroke);
          j++;
        } else {
          break;
        }
      }

      // 複数のポイントがある場合は、バッチコマンドとして最適化
      if (points.length > 1) {
        // 注: バッチストロークコマンドがRust側でサポートされている場合
        // optimized.push({ continueStrokeBatch: points });
        // 現在はそのまま追加
        points.forEach(p => optimized.push({ continueStroke: p }));
      } else {
        optimized.push(current);
      }

      i = j;
    } else {
      optimized.push(current);
      i++;
    }
  }

  return optimized;
}