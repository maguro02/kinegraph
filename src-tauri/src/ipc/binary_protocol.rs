use serde::{Deserialize, Serialize};
use crate::drawing_engine::commands::{DrawEngineCommand, BrushSettings};
use crate::drawing_engine::buffer::BufferManager;

/// バイナリメッセージのラッパー構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryMessage {
    /// メッセージタイプ
    pub message_type: String,
    /// バイナリデータ
    pub data: Vec<u8>,
}

/// DrawEngineCommandのバイナリ表現
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawCommandBinary {
    pub command_type: u8, // コマンドタイプのインデックス
    pub data: Vec<u8>,    // 各コマンドタイプ固有のデータ
}

impl DrawCommandBinary {
    /// DrawEngineCommandからバイナリ表現に変換
    pub fn from_draw_command(command: &DrawEngineCommand) -> Result<Self, bincode::Error> {
        match command {
            DrawEngineCommand::BeginStroke { x, y, pressure } => {
                #[derive(Serialize)]
                struct BeginStrokeData {
                    x: f32,
                    y: f32,
                    pressure: f32,
                }
                let data = bincode::serialize(&BeginStrokeData {
                    x: *x,
                    y: *y,
                    pressure: *pressure,
                })?;
                Ok(DrawCommandBinary {
                    command_type: 0,
                    data,
                })
            }
            DrawEngineCommand::ContinueStroke { x, y, pressure } => {
                #[derive(Serialize)]
                struct ContinueStrokeData {
                    x: f32,
                    y: f32,
                    pressure: f32,
                }
                let data = bincode::serialize(&ContinueStrokeData {
                    x: *x,
                    y: *y,
                    pressure: *pressure,
                })?;
                Ok(DrawCommandBinary {
                    command_type: 1,
                    data,
                })
            }
            DrawEngineCommand::EndStroke => {
                Ok(DrawCommandBinary {
                    command_type: 2,
                    data: vec![],
                })
            }
            DrawEngineCommand::Clear => {
                Ok(DrawCommandBinary {
                    command_type: 3,
                    data: vec![],
                })
            }
            DrawEngineCommand::SetBrush(brush_settings) => {
                let data = bincode::serialize(brush_settings)?;
                Ok(DrawCommandBinary {
                    command_type: 4,
                    data,
                })
            }
            DrawEngineCommand::SetActiveLayer(layer_index) => {
                let data = bincode::serialize(layer_index)?;
                Ok(DrawCommandBinary {
                    command_type: 5,
                    data,
                })
            }
            DrawEngineCommand::CreateLayer => {
                Ok(DrawCommandBinary {
                    command_type: 6,
                    data: vec![],
                })
            }
            DrawEngineCommand::DeleteLayer(layer_index) => {
                let data = bincode::serialize(layer_index)?;
                Ok(DrawCommandBinary {
                    command_type: 7,
                    data,
                })
            }
        }
    }

    /// バイナリ表現からDrawCommandに変換
    pub fn to_draw_command(&self) -> Result<DrawEngineCommand, bincode::Error> {
        match self.command_type {
            0 => {
                #[derive(Deserialize)]
                struct BeginStrokeData {
                    x: f32,
                    y: f32,
                    pressure: f32,
                }
                let data: BeginStrokeData = bincode::deserialize(&self.data)?;
                Ok(DrawEngineCommand::BeginStroke {
                    x: data.x,
                    y: data.y,
                    pressure: data.pressure,
                })
            }
            1 => {
                #[derive(Deserialize)]
                struct ContinueStrokeData {
                    x: f32,
                    y: f32,
                    pressure: f32,
                }
                let data: ContinueStrokeData = bincode::deserialize(&self.data)?;
                Ok(DrawEngineCommand::ContinueStroke {
                    x: data.x,
                    y: data.y,
                    pressure: data.pressure,
                })
            }
            2 => Ok(DrawEngineCommand::EndStroke),
            3 => Ok(DrawEngineCommand::Clear),
            4 => {
                let brush_settings: BrushSettings = bincode::deserialize(&self.data)?;
                Ok(DrawEngineCommand::SetBrush(brush_settings))
            }
            5 => {
                let layer_index: usize = bincode::deserialize(&self.data)?;
                Ok(DrawEngineCommand::SetActiveLayer(layer_index))
            }
            6 => Ok(DrawEngineCommand::CreateLayer),
            7 => {
                let layer_index: usize = bincode::deserialize(&self.data)?;
                Ok(DrawEngineCommand::DeleteLayer(layer_index))
            }
            _ => Err(bincode::Error::new(bincode::ErrorKind::Custom(
                format!("Unknown command type: {}", self.command_type),
            ))),
        }
    }
}

/// キャンバスのイメージデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageDataBinary {
    pub x: f32,
    pub y: f32,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // RGBA形式のピクセルデータ
}

/// ストロークパスのバイナリ表現
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrokePathBinary {
    pub points: Vec<(f32, f32, f32)>, // x, y, pressure
    pub color: [f32; 4],
    pub width: f32,
    pub opacity: f32,
}

/// 描画コマンドのバッチ処理用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawCommandBatch {
    pub commands: Vec<DrawCommandBinary>,
    pub timestamp: u64,
}

impl DrawCommandBatch {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    pub fn add_command(&mut self, command: DrawEngineCommand) -> Result<(), bincode::Error> {
        let binary_command = DrawCommandBinary::from_draw_command(&command)?;
        self.commands.push(binary_command);
        Ok(())
    }

    pub fn to_commands(&self) -> Result<Vec<DrawEngineCommand>, bincode::Error> {
        self.commands
            .iter()
            .map(|cmd| cmd.to_draw_command())
            .collect()
    }
}

/// コマンドタイプの列挙型
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum CommandType {
    BeginStroke = 0,
    ContinueStroke = 1,
    EndStroke = 2,
    Clear = 3,
    SetBrush = 4,
    SetActiveLayer = 5,
    CreateLayer = 6,
    DeleteLayer = 7,
}

/// バイナリプロトコルのヘルパー関数
pub struct BinaryProtocol;

impl BinaryProtocol {
    /// DrawEngineCommandをバイナリデータにエンコード
    pub fn encode_draw_command(command: &DrawEngineCommand) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let binary_command = DrawCommandBinary::from_draw_command(command)?;
        Ok(bincode::serialize(&binary_command)?)
    }
    
    /// バイナリデータからDrawCommandとCommandTypeをデコード
    pub fn decode_draw_command(data: &[u8]) -> Result<(CommandType, DrawEngineCommand), Box<dyn std::error::Error>> {
        let binary_command: DrawCommandBinary = bincode::deserialize(data)?;
        let command_type = match binary_command.command_type {
            0 => CommandType::BeginStroke,
            1 => CommandType::ContinueStroke,
            2 => CommandType::EndStroke,
            3 => CommandType::Clear,
            4 => CommandType::SetBrush,
            5 => CommandType::SetActiveLayer,
            6 => CommandType::CreateLayer,
            7 => CommandType::DeleteLayer,
            _ => return Err("Unknown command type".into()),
        };
        let command = binary_command.to_draw_command()?;
        Ok((command_type, command))
    }
    
    /// コマンドバッチをエンコード
    pub fn encode_command_batch(commands: &[DrawEngineCommand]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut batch = DrawCommandBatch::new();
        for command in commands {
            batch.add_command(command.clone())?;
        }
        Ok(bincode::serialize(&batch)?)
    }
    
    /// コマンドバッチをデコード
    pub fn decode_command_batch(data: &[u8]) -> Result<Vec<DrawEngineCommand>, Box<dyn std::error::Error>> {
        let batch: DrawCommandBatch = bincode::deserialize(data)?;
        Ok(batch.to_commands()?)
    }
    
    /// 画像データをバイナリフォーマットにエンコード
    pub fn encode_image_data(data: &[u8], width: u32, height: u32) -> Vec<u8> {
        let mut result = Vec::new();
        
        // ヘッダー: width (4 bytes), height (4 bytes), compressed (1 byte = 0)
        result.extend_from_slice(&width.to_le_bytes());
        result.extend_from_slice(&height.to_le_bytes());
        result.push(0); // not compressed
        
        // データ
        result.extend_from_slice(data);
        
        result
    }
    
    /// 圧縮された画像データをエンコード
    pub fn encode_image_data_compressed(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let compressed_data = BufferManager::compress(data)?;
        
        let mut result = Vec::new();
        
        // ヘッダー: width (4 bytes), height (4 bytes), compressed (1 byte = 1)
        result.extend_from_slice(&width.to_le_bytes());
        result.extend_from_slice(&height.to_le_bytes());
        result.push(1); // compressed
        
        // 圧縮されたデータ
        result.extend_from_slice(&compressed_data);
        
        Ok(result)
    }
    
    /// RenderResultをバイナリフォーマットにエンコード
    pub fn encode_render_result(canvas_id: &str, image_data: &[u8], width: u32, height: u32, timestamp: u64) -> Vec<u8> {
        let mut result = Vec::new();
        
        // Canvas ID長さ (4 bytes) + Canvas ID
        let id_bytes = canvas_id.as_bytes();
        result.extend_from_slice(&(id_bytes.len() as u32).to_le_bytes());
        result.extend_from_slice(id_bytes);
        
        // タイムスタンプ (8 bytes)
        result.extend_from_slice(&timestamp.to_le_bytes());
        
        // 画像データ
        result.extend_from_slice(&Self::encode_image_data(image_data, width, height));
        
        result
    }
    
    /// 圧縮されたRenderResultをエンコード
    pub fn encode_render_result_compressed(
        canvas_id: &str,
        image_data: &[u8],
        width: u32,
        height: u32,
        timestamp: u64
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut result = Vec::new();
        
        // Canvas ID長さ (4 bytes) + Canvas ID
        let id_bytes = canvas_id.as_bytes();
        result.extend_from_slice(&(id_bytes.len() as u32).to_le_bytes());
        result.extend_from_slice(id_bytes);
        
        // タイムスタンプ (8 bytes)
        result.extend_from_slice(&timestamp.to_le_bytes());
        
        // 圧縮された画像データ
        result.extend_from_slice(&Self::encode_image_data_compressed(image_data, width, height)?);
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawing_engine::commands::{BrushType, BlendMode};

    #[test]
    fn test_draw_command_binary_conversion() {
        // BeginStrokeコマンドのテスト
        let command = DrawEngineCommand::BeginStroke {
            x: 10.0,
            y: 20.0,
            pressure: 0.5,
        };
        let binary = DrawCommandBinary::from_draw_command(&command).unwrap();
        
        // DrawCommandBinaryのシリアライズをテスト
        let serialized = bincode::serialize(&binary).unwrap();
        println!("DrawCommandBinary serialized: {} bytes", serialized.len());
        println!("Bytes: {:02x?}", serialized);
        
        let restored = binary.to_draw_command().unwrap();
        
        match (&command, &restored) {
            (
                DrawEngineCommand::BeginStroke { x: x1, y: y1, pressure: p1 },
                DrawEngineCommand::BeginStroke { x: x2, y: y2, pressure: p2 }
            ) => {
                assert_eq!(x1, x2);
                assert_eq!(y1, y2);
                assert_eq!(p1, p2);
            }
            _ => panic!("Command type mismatch"),
        }
    }

    #[test]
    fn test_brush_settings_serialization() {
        let brush = BrushSettings {
            size: 10.0,
            opacity: 0.8,
            color: [1.0, 0.0, 0.0, 1.0],
            brush_type: BrushType::Pen,
            blend_mode: BlendMode::Normal,
        };
        
        let command = DrawEngineCommand::SetBrush(brush.clone());
        let binary = DrawCommandBinary::from_draw_command(&command).unwrap();
        let restored = binary.to_draw_command().unwrap();
        
        match restored {
            DrawEngineCommand::SetBrush(restored_brush) => {
                assert_eq!(brush.size, restored_brush.size);
                assert_eq!(brush.opacity, restored_brush.opacity);
                assert_eq!(brush.color, restored_brush.color);
            }
            _ => panic!("Command type mismatch"),
        }
    }

    #[test]
    fn test_command_batch() {
        let mut batch = DrawCommandBatch::new();
        
        batch.add_command(DrawEngineCommand::BeginStroke {
            x: 0.0,
            y: 0.0,
            pressure: 1.0,
        }).unwrap();
        
        batch.add_command(DrawEngineCommand::ContinueStroke {
            x: 10.0,
            y: 10.0,
            pressure: 0.8,
        }).unwrap();
        
        batch.add_command(DrawEngineCommand::EndStroke).unwrap();
        
        let commands = batch.to_commands().unwrap();
        assert_eq!(commands.len(), 3);
        
        // バイナリエンコードをテスト
        let encoded = bincode::serialize(&batch).unwrap();
        println!("Encoded batch size: {} bytes", encoded.len());
        println!("First 32 bytes: {:02x?}", &encoded[..encoded.len().min(32)]);
        
        // デコードをテスト
        let decoded: DrawCommandBatch = bincode::deserialize(&encoded).unwrap();
        assert_eq!(decoded.commands.len(), 3);
    }

    #[test]
    fn test_binary_message_serialization() {
        let message = BinaryMessage {
            message_type: "draw_command".to_string(),
            data: vec![1, 2, 3, 4, 5],
        };
        
        let serialized = bincode::serialize(&message).unwrap();
        let deserialized: BinaryMessage = bincode::deserialize(&serialized).unwrap();
        
        assert_eq!(message.message_type, deserialized.message_type);
        assert_eq!(message.data, deserialized.data);
    }
}