import type { DrawCommand, Point, Rect, Transform } from "../hybridCommands";

/**
 * 描画レンダラーのインターフェース
 */
export interface IRenderer {
  init(canvas: HTMLCanvasElement): Promise<void>;
  executeCommands(commands: DrawCommand[]): void;
  clear(): void;
  destroy(): void;
}

/**
 * Canvas 2D Contextを使用したレンダラー
 * （後でWebGPU/WebGLレンダラーに置き換え可能）
 */
export class Canvas2DRenderer implements IRenderer {
  private canvas: HTMLCanvasElement | null = null;
  private ctx: CanvasRenderingContext2D | null = null;
  private layers: Map<string, HTMLCanvasElement> = new Map();
  private layerOrder: string[] = [];

  async init(canvas: HTMLCanvasElement): Promise<void> {
    this.canvas = canvas;
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      throw new Error("Failed to get 2D context");
    }
    this.ctx = ctx;

    // アンチエイリアスの設定
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = "high";
  }

  executeCommands(commands: DrawCommand[]): void {
    for (const command of commands) {
      this.executeCommand(command);
    }
    this.composite();
  }

  private executeCommand(command: DrawCommand): void {
    switch (command.type) {
      case "ClearCanvas":
        this.clear();
        break;

      case "DrawPath":
        this.drawPath(command.payload);
        break;

      case "UpdateRasterArea":
        this.updateRasterArea(command.payload);
        break;

      case "AddLayer":
        this.addLayer(command.payload);
        break;

      case "RemoveLayer":
        this.removeLayer(command.payload);
        break;

      case "ReorderLayers":
        this.reorderLayers(command.payload);
        break;

      case "UpdateLayerProperties":
        this.updateLayerProperties(command.payload);
        break;

      case "ShowSelection":
        // TODO: 選択範囲の表示実装
        break;

      case "ClearSelection":
        // TODO: 選択範囲のクリア実装
        break;

      case "ApplyTransform":
        this.applyTransform(command.payload);
        break;

      case "Batch":
        // バッチコマンドの実行
        for (const cmd of command.payload.commands) {
          this.executeCommand(cmd);
        }
        break;
    }
  }

  private drawPath(payload: {
    points: Point[];
    color: string;
    width: number;
    layer_id: string;
  }): void {
    const layerCanvas = this.getOrCreateLayer(payload.layer_id);
    const ctx = layerCanvas.getContext("2d");
    if (!ctx) return;

    const { points, color, width } = payload;
    if (points.length < 2) return;

    ctx.strokeStyle = color;
    ctx.lineWidth = width;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";

    ctx.beginPath();
    ctx.moveTo(points[0].x, points[0].y);

    for (let i = 1; i < points.length; i++) {
      ctx.lineTo(points[i].x, points[i].y);
    }

    ctx.stroke();
  }

  private updateRasterArea(payload: {
    rect: Rect;
    pixel_data: number[];
    layer_id: string;
  }): void {
    const layerCanvas = this.getOrCreateLayer(payload.layer_id);
    const ctx = layerCanvas.getContext("2d");
    if (!ctx) return;

    const { rect, pixel_data } = payload;
    
    // pixel_dataをImageDataに変換
    const imageData = new ImageData(
      new Uint8ClampedArray(pixel_data),
      rect.width,
      rect.height
    );

    ctx.putImageData(imageData, rect.x, rect.y);
  }

  private addLayer(payload: { layer_id: string; index: number }): void {
    const { layer_id, index } = payload;
    
    if (!this.layers.has(layer_id)) {
      const layerCanvas = this.createLayerCanvas();
      this.layers.set(layer_id, layerCanvas);
    }

    // レイヤー順序に追加
    this.layerOrder = this.layerOrder.filter(id => id !== layer_id);
    this.layerOrder.splice(index, 0, layer_id);
  }

  private removeLayer(payload: { layer_id: string }): void {
    const { layer_id } = payload;
    this.layers.delete(layer_id);
    this.layerOrder = this.layerOrder.filter(id => id !== layer_id);
  }

  private reorderLayers(payload: { layer_ids: string[] }): void {
    this.layerOrder = payload.layer_ids;
  }

  private updateLayerProperties(payload: {
    layer_id: string;
    opacity: number;
    blend_mode: string;
    visible: boolean;
  }): void {
    // レイヤーのプロパティを保存（後で合成時に使用）
    const layer = this.layers.get(payload.layer_id);
    if (layer) {
      // データ属性として保存
      layer.dataset.opacity = payload.opacity.toString();
      layer.dataset.blendMode = payload.blend_mode;
      layer.dataset.visible = payload.visible.toString();
    }
  }

  private applyTransform(payload: {
    layer_id: string;
    transform: Transform;
  }): void {
    const layerCanvas = this.layers.get(payload.layer_id);
    if (!layerCanvas) return;

    const ctx = layerCanvas.getContext("2d");
    if (!ctx) return;

    const { transform } = payload;

    // 一時的なキャンバスに現在の内容をコピー
    const tempCanvas = document.createElement("canvas");
    tempCanvas.width = layerCanvas.width;
    tempCanvas.height = layerCanvas.height;
    const tempCtx = tempCanvas.getContext("2d");
    if (!tempCtx) return;

    tempCtx.drawImage(layerCanvas, 0, 0);

    // レイヤーキャンバスをクリアして変形を適用
    ctx.clearRect(0, 0, layerCanvas.width, layerCanvas.height);
    ctx.save();

    // 変形の中心点（キャンバスの中心）
    const centerX = layerCanvas.width / 2;
    const centerY = layerCanvas.height / 2;

    ctx.translate(centerX + transform.translate_x, centerY + transform.translate_y);
    ctx.rotate(transform.rotation);
    ctx.scale(transform.scale_x, transform.scale_y);
    ctx.translate(-centerX, -centerY);

    // 変形された内容を描画
    ctx.drawImage(tempCanvas, 0, 0);
    ctx.restore();
  }

  private getOrCreateLayer(layerId: string): HTMLCanvasElement {
    let layer = this.layers.get(layerId);
    if (!layer) {
      layer = this.createLayerCanvas();
      this.layers.set(layerId, layer);
      this.layerOrder.push(layerId);
    }
    return layer;
  }

  private createLayerCanvas(): HTMLCanvasElement {
    const canvas = document.createElement("canvas");
    canvas.width = this.canvas?.width || 1920;
    canvas.height = this.canvas?.height || 1080;
    
    // 背景を透明に設定
    const ctx = canvas.getContext("2d");
    if (ctx) {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
    }
    
    return canvas;
  }

  /**
   * すべてのレイヤーを合成してメインキャンバスに描画
   */
  private composite(): void {
    if (!this.ctx || !this.canvas) return;

    // メインキャンバスをクリア
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);

    // レイヤーを順番に合成
    for (const layerId of this.layerOrder) {
      const layer = this.layers.get(layerId);
      if (!layer) continue;

      const opacity = parseFloat(layer.dataset.opacity || "1");
      const blendMode = layer.dataset.blendMode || "normal";
      const visible = layer.dataset.visible !== "false";

      if (!visible) continue;

      this.ctx.save();
      this.ctx.globalAlpha = opacity;
      this.ctx.globalCompositeOperation = this.getCompositeOperation(blendMode);
      this.ctx.drawImage(layer, 0, 0);
      this.ctx.restore();
    }
  }

  private getCompositeOperation(blendMode: string): GlobalCompositeOperation {
    const modeMap: Record<string, GlobalCompositeOperation> = {
      normal: "source-over",
      multiply: "multiply",
      screen: "screen",
      overlay: "overlay",
    };
    return modeMap[blendMode] || "source-over";
  }

  clear(): void {
    if (!this.ctx || !this.canvas) return;
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    
    // すべてのレイヤーもクリア
    for (const layer of this.layers.values()) {
      const ctx = layer.getContext("2d");
      if (ctx) {
        ctx.clearRect(0, 0, layer.width, layer.height);
      }
    }
  }

  destroy(): void {
    this.layers.clear();
    this.layerOrder = [];
    this.ctx = null;
    this.canvas = null;
  }
}