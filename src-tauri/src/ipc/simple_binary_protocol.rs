use crate::drawing_engine::commands::{DrawEngineCommand, BrushSettings, BrushType, BlendMode};
use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt};

pub struct SimpleBinaryProtocol;

impl SimpleBinaryProtocol {
    // コマンドタイプの定数
    const CMD_BEGIN_STROKE: u8 = 0;
    const CMD_CONTINUE_STROKE: u8 = 1;
    const CMD_END_STROKE: u8 = 2;
    const CMD_CLEAR: u8 = 3;
    const CMD_SET_BRUSH: u8 = 4;
    const CMD_SET_ACTIVE_LAYER: u8 = 5;
    const CMD_CREATE_LAYER: u8 = 6;
    const CMD_DELETE_LAYER: u8 = 7;
    
    /// 単一のコマンドをデコード
    pub fn decode_command(data: &[u8]) -> Result<DrawEngineCommand, Box<dyn std::error::Error>> {
        if data.is_empty() {
            return Err("Empty data".into());
        }
        
        let mut cursor = Cursor::new(data);
        let cmd_type = cursor.read_u8()?;
        
        match cmd_type {
            Self::CMD_BEGIN_STROKE => {
                let x = cursor.read_f32::<LittleEndian>()?;
                let y = cursor.read_f32::<LittleEndian>()?;
                let pressure = cursor.read_f32::<LittleEndian>()?;
                Ok(DrawEngineCommand::BeginStroke { x, y, pressure })
            }
            
            Self::CMD_CONTINUE_STROKE => {
                let x = cursor.read_f32::<LittleEndian>()?;
                let y = cursor.read_f32::<LittleEndian>()?;
                let pressure = cursor.read_f32::<LittleEndian>()?;
                Ok(DrawEngineCommand::ContinueStroke { x, y, pressure })
            }
            
            Self::CMD_END_STROKE => {
                Ok(DrawEngineCommand::EndStroke)
            }
            
            Self::CMD_CLEAR => {
                Ok(DrawEngineCommand::Clear)
            }
            
            Self::CMD_SET_BRUSH => {
                let size = cursor.read_f32::<LittleEndian>()?;
                let opacity = cursor.read_f32::<LittleEndian>()?;
                
                let mut color = [0.0; 4];
                for i in 0..4 {
                    color[i] = cursor.read_f32::<LittleEndian>()?;
                }
                
                let brush_type_val = cursor.read_u32::<LittleEndian>()?;
                let brush_type = match brush_type_val {
                    0 => BrushType::Pen,
                    1 => BrushType::Brush,
                    2 => BrushType::Eraser,
                    _ => return Err(format!("Invalid brush type: {}", brush_type_val).into()),
                };
                
                let blend_mode_val = cursor.read_u32::<LittleEndian>()?;
                let blend_mode = match blend_mode_val {
                    0 => BlendMode::Normal,
                    1 => BlendMode::Multiply,
                    2 => BlendMode::Screen,
                    3 => BlendMode::Overlay,
                    _ => return Err(format!("Invalid blend mode: {}", blend_mode_val).into()),
                };
                
                Ok(DrawEngineCommand::SetBrush(BrushSettings {
                    size,
                    opacity,
                    color,
                    brush_type,
                    blend_mode,
                }))
            }
            
            Self::CMD_SET_ACTIVE_LAYER => {
                let layer_index = cursor.read_u64::<LittleEndian>()? as usize;
                Ok(DrawEngineCommand::SetActiveLayer(layer_index))
            }
            
            Self::CMD_CREATE_LAYER => {
                Ok(DrawEngineCommand::CreateLayer)
            }
            
            Self::CMD_DELETE_LAYER => {
                let layer_index = cursor.read_u64::<LittleEndian>()? as usize;
                Ok(DrawEngineCommand::DeleteLayer(layer_index))
            }
            
            _ => Err(format!("Unknown command type: {}", cmd_type).into()),
        }
    }
    
    /// バッチコマンドをデコード
    pub fn decode_commands(data: &[u8]) -> Result<Vec<DrawEngineCommand>, Box<dyn std::error::Error>> {
        let mut cursor = Cursor::new(data);
        let command_count = cursor.read_u32::<LittleEndian>()? as usize;
        
        let mut commands = Vec::with_capacity(command_count);
        
        for _ in 0..command_count {
            let cmd_length = cursor.read_u32::<LittleEndian>()? as usize;
            
            // 現在の位置を取得
            let current_pos = cursor.position() as usize;
            
            // コマンドデータを取得
            if current_pos + cmd_length > data.len() {
                return Err("Insufficient data for command".into());
            }
            
            let cmd_data = &data[current_pos..current_pos + cmd_length];
            let command = Self::decode_command(cmd_data)?;
            commands.push(command);
            
            // カーソルを進める
            cursor.set_position((current_pos + cmd_length) as u64);
        }
        
        Ok(commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_decode_begin_stroke() {
        let data = vec![0, 0, 0, 128, 63, 0, 0, 0, 64, 0, 0, 0, 63]; // cmd=0, x=1.0, y=2.0, pressure=0.5
        let command = SimpleBinaryProtocol::decode_command(&data).unwrap();
        
        match command {
            DrawEngineCommand::BeginStroke { x, y, pressure } => {
                assert_eq!(x, 1.0);
                assert_eq!(y, 2.0);
                assert_eq!(pressure, 0.5);
            }
            _ => panic!("Wrong command type"),
        }
    }
    
    #[test]
    fn test_decode_commands_batch() {
        // バッチ: 2コマンド
        let mut data = vec![];
        
        // コマンド数 (2)
        data.extend_from_slice(&2u32.to_le_bytes());
        
        // コマンド1: beginStroke
        let cmd1 = vec![0, 0, 0, 128, 63, 0, 0, 0, 64, 0, 0, 0, 63]; // cmd=0, x=1.0, y=2.0, pressure=0.5
        data.extend_from_slice(&(cmd1.len() as u32).to_le_bytes());
        data.extend_from_slice(&cmd1);
        
        // コマンド2: endStroke
        let cmd2 = vec![2]; // cmd=2
        data.extend_from_slice(&(cmd2.len() as u32).to_le_bytes());
        data.extend_from_slice(&cmd2);
        
        let commands = SimpleBinaryProtocol::decode_commands(&data).unwrap();
        assert_eq!(commands.len(), 2);
        
        match &commands[0] {
            DrawEngineCommand::BeginStroke { x, y, pressure } => {
                assert_eq!(*x, 1.0);
                assert_eq!(*y, 2.0);
                assert_eq!(*pressure, 0.5);
            }
            _ => panic!("Wrong command type for first command"),
        }
        
        match &commands[1] {
            DrawEngineCommand::EndStroke => {},
            _ => panic!("Wrong command type for second command"),
        }
    }
}