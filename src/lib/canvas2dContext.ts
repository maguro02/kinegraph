import type { DrawingContext, Point, StrokeId, Color, BrushTool } from '../types/drawing.ts';

/**
 * Canvas 2D APIを使用した描画コンテキスト実装
 * WASMが利用できない場合のフォールバック
 */
export class Canvas2DDrawingContext implements DrawingContext {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private width: number;
  private height: number;
  private strokeIdCounter = 0;
  private activeStrokes = new Map<StrokeId, { points: Point[], tool: BrushTool, color: Color }>();

  constructor(canvas: HTMLCanvasElement, width: number, height: number) {
    this.canvas = canvas;
    this.width = width;
    this.height = height;

    const ctx = canvas.getContext('2d');
    if (!ctx) {
      throw new Error('Failed to get 2D context');
    }
    this.ctx = ctx;

    // キャンバスサイズ設定
    canvas.width = width;
    canvas.height = height;

    // 初期設定
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = 'high';
  }

  async initialize(): Promise<void> {
    // Canvas 2Dは追加の初期化は不要
    await Promise.resolve();
  }

  async startStroke(point: Point, tool: BrushTool, color: Color): Promise<StrokeId> {
    const strokeId = (this.strokeIdCounter++).toString();
    
    // ストロークデータを保存
    this.activeStrokes.set(strokeId, {
      points: [point],
      tool,
      color
    });

    // 最初の点を描画
    this.drawPoint(point, tool, color);
    
    return strokeId;
  }

  async addPoint(strokeId: StrokeId, point: Point): Promise<void> {
    const strokeData = this.activeStrokes.get(strokeId);
    if (!strokeData) {
      throw new Error(`Stroke ${strokeId} not found`);
    }

    // 前の点から新しい点まで線を描画
    const lastPoint = strokeData.points[strokeData.points.length - 1];
    this.drawLine(lastPoint, point, strokeData.tool, strokeData.color);

    // 点を追加
    strokeData.points.push(point);
  }

  async endStroke(strokeId: StrokeId): Promise<void> {
    // ストロークデータをクリア
    this.activeStrokes.delete(strokeId);
  }

  async clear(): Promise<void> {
    this.ctx.clearRect(0, 0, this.width, this.height);
    this.activeStrokes.clear();
  }

  resize(width: number, height: number): void {
    // 現在の内容を保存
    const imageData = this.ctx.getImageData(0, 0, this.width, this.height);
    
    // サイズ変更
    this.width = width;
    this.height = height;
    this.canvas.width = width;
    this.canvas.height = height;
    
    // 設定を再適用
    this.ctx.lineCap = 'round';
    this.ctx.lineJoin = 'round';
    this.ctx.imageSmoothingEnabled = true;
    this.ctx.imageSmoothingQuality = 'high';
    
    // 内容を復元（収まる範囲で）
    this.ctx.putImageData(imageData, 0, 0);
  }

  destroy(): void {
    this.activeStrokes.clear();
  }

  private drawPoint(point: Point, tool: BrushTool, color: Color): void {
    this.ctx.save();
    
    // 色と透明度を設定
    this.ctx.globalAlpha = color.a * tool.opacity;
    this.ctx.fillStyle = `rgba(${color.r}, ${color.g}, ${color.b}, 1)`;
    
    // 点を円として描画
    this.ctx.beginPath();
    this.ctx.arc(point.x, point.y, tool.size / 2, 0, Math.PI * 2);
    this.ctx.fill();
    
    this.ctx.restore();
  }

  private drawLine(from: Point, to: Point, tool: BrushTool, color: Color): void {
    this.ctx.save();
    
    // 色と透明度を設定
    this.ctx.globalAlpha = color.a * tool.opacity;
    this.ctx.strokeStyle = `rgba(${color.r}, ${color.g}, ${color.b}, 1)`;
    this.ctx.lineWidth = tool.size;
    
    // 線を描画
    this.ctx.beginPath();
    this.ctx.moveTo(from.x, from.y);
    this.ctx.lineTo(to.x, to.y);
    this.ctx.stroke();
    
    this.ctx.restore();
  }
}

/**
 * Canvas 2Dコンテキストを作成するファクトリ関数
 */
export function createCanvas2DContext(
  canvas: HTMLCanvasElement,
  width: number,
  height: number
): DrawingContext {
  return new Canvas2DDrawingContext(canvas, width, height);
}