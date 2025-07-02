import type { Point, BrushTool, Color, StrokeId } from './drawing.ts';

// Worker への描画リクエスト
export type DrawRequest = {
  requestId?: string;
} & (
  | {
      type: 'init';
      data: {
        canvasBuffer?: SharedArrayBuffer;
        width: number;
        height: number;
      };
    }
  | {
      type: 'startStroke';
      data: {
        point: Point;
        tool: BrushTool;
        color: Color;
        strokeId: StrokeId;
      };
    }
  | {
      type: 'addPoint';
      data: {
        strokeId: StrokeId;
        point: Point;
      };
    }
  | {
      type: 'endStroke';
      data: {
        strokeId: StrokeId;
      };
    }
  | {
      type: 'clear';
      data: Record<string, never>;
    }
  | {
      type: 'resize';
      data: {
        width: number;
        height: number;
        canvasBuffer: SharedArrayBuffer;
      };
    }
  | {
      type: 'destroy';
      data: Record<string, never>;
    }
);

// Worker からのレスポンス
export type DrawResponse = 
  | {
      type: 'initialized';
      data: {
        success: boolean;
        canvasBuffer?: SharedArrayBuffer;
        sharedBuffer?: SharedArrayBuffer;
      };
    }
  | {
      type: 'strokeStarted';
      data: {
        strokeId: string;
      };
    }
  | {
      type: 'pointAdded';
      data: {
        success: boolean;
      };
    }
  | {
      type: 'strokeEnded';
      data: {
        success: boolean;
      };
    }
  | {
      type: 'cleared';
      data: {
        success: boolean;
      };
    }
  | {
      type: 'resized';
      data: {
        success: boolean;
      };
    }
  | {
      type: 'destroyed';
      data: {
        success: boolean;
      };
    }
  | {
      type: 'rendered';
      data: {
        timestamp: number;
      };
    }
  | {
      type: 'error';
      data: {
        message: string;
        originalType: string;
      };
    };