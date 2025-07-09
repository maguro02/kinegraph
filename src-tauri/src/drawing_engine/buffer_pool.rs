use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use wgpu::{Buffer, Device, BufferDescriptor, BufferUsages};

/// バッファプール - バッファの再利用を管理
pub struct BufferPool {
    device: Arc<Device>,
    available_buffers: Mutex<VecDeque<(Buffer, u64)>>, // (buffer, size)
}

impl BufferPool {
    pub fn new(device: Arc<Device>) -> Self {
        Self {
            device,
            available_buffers: Mutex::new(VecDeque::new()),
        }
    }
    
    /// 指定サイズのバッファを取得（再利用可能なものがあれば再利用）
    pub fn get_buffer(&self, size: u64) -> Buffer {
        let mut buffers = self.available_buffers.lock().unwrap();
        
        // 再利用可能なバッファを探す
        if let Some(index) = buffers.iter().position(|(_, buffer_size)| *buffer_size >= size) {
            let (buffer, _) = buffers.remove(index).unwrap();
            return buffer;
        }
        
        // 新しいバッファを作成
        self.device.create_buffer(&BufferDescriptor {
            label: Some("Pooled Output Buffer"),
            size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        })
    }
    
    /// バッファをプールに返却
    pub fn return_buffer(&self, buffer: Buffer, size: u64) {
        let mut buffers = self.available_buffers.lock().unwrap();
        
        // プールサイズの上限（例：10個まで）
        if buffers.len() < 10 {
            buffers.push_back((buffer, size));
        }
        // 上限を超えたらバッファは破棄される（Dropトレイトによって）
    }
    
    /// プールをクリア
    pub fn clear(&self) {
        let mut buffers = self.available_buffers.lock().unwrap();
        buffers.clear();
    }
}