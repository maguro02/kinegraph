// バイナリプロトコルのTypeScript実装

import type { DrawEngineCommand } from './bindings';

export interface ImageHeader {
  width: number;
  height: number;
  format: string;
  compressed: boolean;
  originalSize: number;
  compressedSize?: number;
}

export interface ImageChunk {
  chunkIndex: number;
  totalChunks: number;
  data: Uint8Array;
}

// コマンドタイプの列挙（Rust側と同じ値）
export enum CommandType {
  BeginStroke = 0,
  ContinueStroke = 1,
  EndStroke = 2,
  Clear = 3,
  SetBrush = 4,
  SetActiveLayer = 5,
  CreateLayer = 6,
  DeleteLayer = 7,
}

// 描画コマンドのバイナリ表現
export interface DrawCommandBinary {
  commandType: number;
  data: Uint8Array;
}

// 描画コマンドのバッチ
export interface DrawCommandBatch {
  commands: DrawCommandBinary[];
  timestamp: number;
}

export class BinaryProtocol {
  // DrawEngineCommandをバイナリ形式にエンコード（Rust側のenum形式と一致）
  static encodeDrawCommand(command: DrawEngineCommand): DrawCommandBinary {
    // beginStroke - variant index 0
    if (typeof command === 'object' && 'beginStroke' in command) {
      const { x, y, pressure } = command.beginStroke;
      const buffer = new ArrayBuffer(12); // 3 * 4 bytes for Float32
      const view = new DataView(buffer);
      view.setFloat32(0, x, true); // little-endian
      view.setFloat32(4, y, true);
      view.setFloat32(8, pressure, true);
      return {
        commandType: CommandType.BeginStroke,
        data: new Uint8Array(buffer),
      };
    }
    
    // continueStroke
    if (typeof command === 'object' && 'continueStroke' in command) {
      const { x, y, pressure } = command.continueStroke;
      const buffer = new ArrayBuffer(12); // 3 * 4 bytes for Float32
      const view = new DataView(buffer);
      view.setFloat32(0, x, true); // little-endian
      view.setFloat32(4, y, true);
      view.setFloat32(8, pressure, true);
      return {
        commandType: CommandType.ContinueStroke,
        data: new Uint8Array(buffer),
      };
    }
    
    // endStroke
    if (command === 'endStroke') {
      return {
        commandType: CommandType.EndStroke,
        data: new Uint8Array(0),
      };
    }
    
    // clear
    if (command === 'clear') {
      return {
        commandType: CommandType.Clear,
        data: new Uint8Array(0),
      };
    }
    
    // setBrush
    if (typeof command === 'object' && 'setBrush' in command) {
      const brush = command.setBrush;
      // BrushSettingsのバイナリエンコード（bincode形式）
      // size(4) + opacity(4) + color(16) + brushType(4: enumはRustでu32) + blendMode(4) = 32 bytes
      const buffer = new ArrayBuffer(32);
      const view = new DataView(buffer);
      let offset = 0;
      
      // size: f32
      view.setFloat32(offset, brush.size, true);
      offset += 4;
      
      // opacity: f32
      view.setFloat32(offset, brush.opacity, true);
      offset += 4;
      
      // color: [f32; 4] (RGBA)
      for (let i = 0; i < 4; i++) {
        view.setFloat32(offset, brush.color[i], true);
        offset += 4;
      }
      
      // brushType: enum (bincodeではu32としてエンコード)
      const brushTypeValue = brush.brushType === 'pen' ? 0 : brush.brushType === 'brush' ? 1 : 2; // eraser = 2
      view.setUint32(offset, brushTypeValue, true);
      offset += 4;
      
      // blendMode: enum (bincodeではu32としてエンコード)
      const blendModeValue = 
        brush.blendMode === 'normal' ? 0 :
        brush.blendMode === 'multiply' ? 1 :
        brush.blendMode === 'screen' ? 2 : 3; // overlay = 3
      view.setUint32(offset, blendModeValue, true);
      
      return {
        commandType: CommandType.SetBrush,
        data: new Uint8Array(buffer),
      };
    }
    
    // setActiveLayer
    if (typeof command === 'object' && 'setActiveLayer' in command) {
      const layerIndex = command.setActiveLayer;
      const buffer = new ArrayBuffer(8); // usize is u64 in bincode on 64-bit systems
      const view = new DataView(buffer);
      // bincodeのusizeはu64としてエンコード
      view.setBigUint64(0, BigInt(layerIndex), true);
      return {
        commandType: CommandType.SetActiveLayer,
        data: new Uint8Array(buffer),
      };
    }
    
    // createLayer
    if (command === 'createLayer') {
      return {
        commandType: CommandType.CreateLayer,
        data: new Uint8Array(0),
      };
    }
    
    // deleteLayer
    if (typeof command === 'object' && 'deleteLayer' in command) {
      const layerIndex = command.deleteLayer;
      const buffer = new ArrayBuffer(8); // usize is u64 in bincode on 64-bit systems
      const view = new DataView(buffer);
      view.setBigUint64(0, BigInt(layerIndex), true);
      return {
        commandType: CommandType.DeleteLayer,
        data: new Uint8Array(buffer),
      };
    }
    
    throw new Error(`Unknown command type: ${JSON.stringify(command)}`);
  }
  
  // DrawCommandBinaryをシリアライズ（bincode形式）
  static serializeDrawCommandBinary(commandBinary: DrawCommandBinary): Uint8Array {
    // bincode形式: command_type(1 byte) + data(Vec<u8>)
    // Vec<u8>はbincodeでは長さをu64（8バイト）としてエンコード + データ
    const dataLength = commandBinary.data.length;
    
    const totalSize = 1 + 8 + dataLength; // 1 byte for command_type, 8 bytes for length, then data
    const buffer = new ArrayBuffer(totalSize);
    const view = new DataView(buffer);
    const uint8View = new Uint8Array(buffer);
    
    let offset = 0;
    
    // command_type: u8
    uint8View[offset] = commandBinary.commandType;
    offset += 1;
    
    // data: Vec<u8> - u64 length (little-endian) + data
    view.setBigUint64(offset, BigInt(dataLength), true);
    offset += 8;
    
    uint8View.set(commandBinary.data, offset);
    
    return uint8View;
  }
  
  
  // 複数のDrawEngineCommandをバッチとしてエンコード（bincode互換）
  static encodeCommandBatch(commands: DrawEngineCommand[]): Uint8Array {
    const batch: DrawCommandBatch = {
      commands: commands.map(cmd => this.encodeDrawCommand(cmd)),
      timestamp: Date.now(),
    };
    
    // バッチのシリアライズ（bincodeフォーマット）
    // DrawCommandBatch { commands: Vec<DrawCommandBinary>, timestamp: u64 }
    
    // まず各コマンドをシリアライズ
    const serializedCommands = batch.commands.map(cmd => this.serializeDrawCommandBinary(cmd));
    
    // 全体のサイズを計算
    const totalCommandsSize = serializedCommands.reduce((sum, cmd) => sum + cmd.length, 0);
    const totalSize = 8 + totalCommandsSize + 8; // 8 bytes for Vec length, commands data, 8 bytes for timestamp
    
    const buffer = new ArrayBuffer(totalSize);
    const view = new DataView(buffer);
    const uint8View = new Uint8Array(buffer);
    
    let offset = 0;
    
    // commands: Vec<DrawCommandBinary> - u64で長さ（little-endian）、その後要素
    view.setBigUint64(offset, BigInt(batch.commands.length), true);
    offset += 8;
    
    // 各コマンドを書き込み
    for (const cmdData of serializedCommands) {
      uint8View.set(cmdData, offset);
      offset += cmdData.length;
    }
    
    // timestamp: u64 (little-endian)
    const timestamp = BigInt(batch.timestamp);
    view.setBigUint64(offset, timestamp, true);
    
    // デバッグ情報を出力
    console.log('[BinaryProtocol] Encoded command batch:', {
      commandsCount: batch.commands.length,
      totalSize: uint8View.length,
      timestampOffset: offset,
      timestamp: batch.timestamp,
      firstBytes: Array.from(uint8View.slice(0, Math.min(32, uint8View.length)))
        .map(b => b.toString(16).padStart(2, '0'))
        .join(' ')
    });
    
    return uint8View;
  }
  
  // 単一のDrawEngineCommandをバイナリ形式にエンコード（bincode互換）
  static encodeDrawCommandDirect(command: DrawEngineCommand): Uint8Array {
    const commandBinary = this.encodeDrawCommand(command);
    const encoded = this.serializeDrawCommandBinary(commandBinary);
    
    // デバッグ情報を出力
    console.log('[BinaryProtocol] Encoded single command:', {
      command: command,
      commandType: commandBinary.commandType,
      dataSize: commandBinary.data.length,
      totalSize: encoded.length,
      firstBytes: Array.from(encoded.slice(0, Math.min(32, encoded.length)))
        .map(b => b.toString(16).padStart(2, '0'))
        .join(' ')
    });
    
    return encoded;
  }

  // 画像データのデコード
  static decodeImageData(buffer: ArrayBuffer): {
    header: ImageHeader;
    data: Uint8Array;
  } {
    const view = new DataView(buffer);
    
    // ヘッダー長を読み取り（最初の4バイト）
    const headerLength = view.getUint32(0, true); // little-endian
    
    // ヘッダーのJSON部分をデコード
    const headerBytes = new Uint8Array(buffer, 4, headerLength);
    const headerJson = new TextDecoder().decode(headerBytes);
    const header: ImageHeader = JSON.parse(headerJson);
    
    // 画像データ部分を取得
    const dataOffset = 4 + headerLength;
    const data = new Uint8Array(buffer, dataOffset);
    
    return { header, data };
  }
  
  // LZ4解凍（スタブ実装）
  static async decompressLZ4(compressed: Uint8Array, _originalSize: number): Promise<Uint8Array> {
    // TODO: 実際のLZ4解凍実装を追加
    console.warn('LZ4 decompression not implemented yet, returning compressed data');
    return compressed;
  }
  
  // 画像データを解凍してRGBAデータとして返す
  static async decodeAndDecompressImage(buffer: ArrayBuffer): Promise<{
    width: number;
    height: number;
    data: Uint8Array;
  }> {
    const { header, data } = this.decodeImageData(buffer);
    
    let imageData: Uint8Array;
    if (header.compressed && header.compressedSize) {
      // 圧縮されている場合は解凍
      imageData = await this.decompressLZ4(data, header.originalSize);
    } else {
      imageData = data;
    }
    
    return {
      width: header.width,
      height: header.height,
      data: imageData,
    };
  }
  
  // チャンクデータを結合
  static combineChunks(chunks: ImageChunk[]): Uint8Array {
    // チャンクをインデックス順にソート
    chunks.sort((a, b) => a.chunkIndex - b.chunkIndex);
    
    // 全体のサイズを計算
    const totalSize = chunks.reduce((sum, chunk) => sum + chunk.data.length, 0);
    const combined = new Uint8Array(totalSize);
    
    // チャンクを結合
    let offset = 0;
    for (const chunk of chunks) {
      combined.set(chunk.data, offset);
      offset += chunk.data.length;
    }
    
    return combined;
  }
}

// パフォーマンス計測用のユーティリティ
export class PerformanceMeasure {
  private startTime: number;
  private measurements: Map<string, number> = new Map();
  
  constructor() {
    this.startTime = performance.now();
  }
  
  mark(label: string) {
    const elapsed = performance.now() - this.startTime;
    this.measurements.set(label, elapsed);
  }
  
  getReport(): Record<string, number> {
    const report: Record<string, number> = {};
    let previousTime = 0;
    
    for (const [label, time] of this.measurements) {
      report[label] = time - previousTime;
      previousTime = time;
    }
    
    report['total'] = performance.now() - this.startTime;
    return report;
  }
}