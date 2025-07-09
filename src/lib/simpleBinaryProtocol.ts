// シンプルなバイナリプロトコルの実装
// 軽量で高速な描画コマンドのエンコード/デコード

import type { DrawEngineCommand } from './bindings';

export class SimpleBinaryProtocol {
  // コマンドタイプの定数
  private static readonly CMD_BEGIN_STROKE = 0;
  private static readonly CMD_CONTINUE_STROKE = 1;
  private static readonly CMD_END_STROKE = 2;
  private static readonly CMD_CLEAR = 3;
  private static readonly CMD_SET_BRUSH = 4;
  private static readonly CMD_SET_ACTIVE_LAYER = 5;
  private static readonly CMD_CREATE_LAYER = 6;
  private static readonly CMD_DELETE_LAYER = 7;

  // 単一のDrawEngineCommandをバイナリ形式にエンコード
  static encodeCommand(command: DrawEngineCommand): Uint8Array {
    // beginStroke
    if (typeof command === 'object' && 'beginStroke' in command) {
      const { x, y, pressure } = command.beginStroke;
      const buffer = new ArrayBuffer(13); // 1 (cmd) + 4*3 (floats)
      const view = new DataView(buffer);
      view.setUint8(0, this.CMD_BEGIN_STROKE);
      view.setFloat32(1, x, true);
      view.setFloat32(5, y, true);
      view.setFloat32(9, pressure, true);
      return new Uint8Array(buffer);
    }
    
    // continueStroke
    if (typeof command === 'object' && 'continueStroke' in command) {
      const { x, y, pressure } = command.continueStroke;
      const buffer = new ArrayBuffer(13); // 1 (cmd) + 4*3 (floats)
      const view = new DataView(buffer);
      view.setUint8(0, this.CMD_CONTINUE_STROKE);
      view.setFloat32(1, x, true);
      view.setFloat32(5, y, true);
      view.setFloat32(9, pressure, true);
      return new Uint8Array(buffer);
    }
    
    // endStroke
    if (command === 'endStroke') {
      const buffer = new ArrayBuffer(1);
      const view = new DataView(buffer);
      view.setUint8(0, this.CMD_END_STROKE);
      return new Uint8Array(buffer);
    }
    
    // clear
    if (command === 'clear') {
      const buffer = new ArrayBuffer(1);
      const view = new DataView(buffer);
      view.setUint8(0, this.CMD_CLEAR);
      return new Uint8Array(buffer);
    }
    
    // setBrush
    if (typeof command === 'object' && 'setBrush' in command) {
      const brush = command.setBrush;
      const buffer = new ArrayBuffer(33); // 1 (cmd) + 32 (brush data)
      const view = new DataView(buffer);
      view.setUint8(0, this.CMD_SET_BRUSH);
      
      let offset = 1;
      view.setFloat32(offset, brush.size, true);
      offset += 4;
      view.setFloat32(offset, brush.opacity, true);
      offset += 4;
      
      // color: [f32; 4]
      for (let i = 0; i < 4; i++) {
        view.setFloat32(offset, brush.color[i], true);
        offset += 4;
      }
      
      // brushType (as u32)
      const brushTypeValue = brush.brushType === 'pen' ? 0 : brush.brushType === 'brush' ? 1 : 2;
      view.setUint32(offset, brushTypeValue, true);
      offset += 4;
      
      // blendMode (as u32)
      const blendModeValue = 
        brush.blendMode === 'normal' ? 0 :
        brush.blendMode === 'multiply' ? 1 :
        brush.blendMode === 'screen' ? 2 : 3;
      view.setUint32(offset, blendModeValue, true);
      
      return new Uint8Array(buffer);
    }
    
    // setActiveLayer
    if (typeof command === 'object' && 'setActiveLayer' in command) {
      const layerIndex = command.setActiveLayer;
      const buffer = new ArrayBuffer(9); // 1 (cmd) + 8 (u64)
      const view = new DataView(buffer);
      view.setUint8(0, this.CMD_SET_ACTIVE_LAYER);
      view.setBigUint64(1, BigInt(layerIndex), true);
      return new Uint8Array(buffer);
    }
    
    // createLayer
    if (command === 'createLayer') {
      const buffer = new ArrayBuffer(1);
      const view = new DataView(buffer);
      view.setUint8(0, this.CMD_CREATE_LAYER);
      return new Uint8Array(buffer);
    }
    
    // deleteLayer
    if (typeof command === 'object' && 'deleteLayer' in command) {
      const layerIndex = command.deleteLayer;
      const buffer = new ArrayBuffer(9); // 1 (cmd) + 8 (u64)
      const view = new DataView(buffer);
      view.setUint8(0, this.CMD_DELETE_LAYER);
      view.setBigUint64(1, BigInt(layerIndex), true);
      return new Uint8Array(buffer);
    }
    
    throw new Error(`Unknown command type: ${JSON.stringify(command)}`);
  }
  
  // 複数のコマンドをバッチとしてエンコード
  static encodeCommandBatch(commands: DrawEngineCommand[]): Uint8Array {
    // まず各コマンドをエンコード
    const encodedCommands = commands.map(cmd => this.encodeCommand(cmd));
    
    // 全体のサイズを計算
    const totalSize = encodedCommands.reduce((sum, cmd) => sum + cmd.length + 4, 4);
    // 4 bytes for command count + (4 bytes length + data) per command
    
    const buffer = new ArrayBuffer(totalSize);
    const view = new DataView(buffer);
    const uint8View = new Uint8Array(buffer);
    
    let offset = 0;
    
    // コマンド数を書き込み
    view.setUint32(offset, commands.length, true);
    offset += 4;
    
    // 各コマンドを書き込み
    for (const cmdData of encodedCommands) {
      // コマンドの長さ
      view.setUint32(offset, cmdData.length, true);
      offset += 4;
      
      // コマンドデータ
      uint8View.set(cmdData, offset);
      offset += cmdData.length;
    }
    
    return uint8View;
  }
}