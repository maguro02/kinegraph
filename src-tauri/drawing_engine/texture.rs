use wgpu::*;
use log::{info, debug, error};
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::error::Error;
use std::fmt;

/// テクスチャ管理のエラー型
#[derive(Debug)]
pub enum TextureError {
    DeviceNotInitialized,
    TextureCreationFailed(String),
    TextureNotFound(String),
    InvalidDimensions(u32, u32),
    BufferCreationFailed(String),
    BufferReadFailed(String),
    MemoryLimitExceeded(u64),
}

impl fmt::Display for TextureError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TextureError::DeviceNotInitialized => {
                write!(f, "wgpu Device が初期化されていません")
            }
            TextureError::TextureCreationFailed(msg) => {
                write!(f, "テクスチャ作成に失敗しました: {}", msg)
            }
            TextureError::TextureNotFound(id) => {
                write!(f, "テクスチャが見つかりません: {}", id)
            }
            TextureError::InvalidDimensions(width, height) => {
                write!(f, "無効な寸法です: {}x{}", width, height)
            }
            TextureError::BufferCreationFailed(msg) => {
                write!(f, "バッファ作成に失敗しました: {}", msg)
            }
            TextureError::BufferReadFailed(msg) => {
                write!(f, "バッファ読み取りに失敗しました: {}", msg)
            }
            TextureError::MemoryLimitExceeded(size) => {
                write!(f, "メモリ使用量が上限を超えました: {} bytes", size)
            }
        }
    }
}

impl Error for TextureError {}

/// テクスチャの仕様を定義
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureSpec {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub usage: TextureUsages,
}

impl Hash for TextureSpec {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.width.hash(state);
        self.height.hash(state);
        // formatとusageは基本的に同じなのでハッシュから除外
    }
}

impl TextureSpec {
    /// レイヤー用の標準テクスチャ仕様
    pub fn layer_texture(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC | TextureUsages::COPY_DST,
        }
    }

    /// テクスチャのメモリ使用量を計算（バイト）
    pub fn memory_size(&self) -> u64 {
        let bytes_per_pixel = match self.format {
            TextureFormat::Rgba8UnormSrgb | TextureFormat::Bgra8UnormSrgb => 4,
            TextureFormat::R8Unorm => 1,
            _ => 4, // 安全のため4を仮定
        };
        (self.width as u64) * (self.height as u64) * bytes_per_pixel
    }
}

/// 管理されたテクスチャ
pub struct ManagedTexture {
    pub texture: Texture,
    pub view: TextureView,
    pub spec: TextureSpec,
    pub last_used: std::time::Instant,
    pub is_in_use: bool,
}

impl ManagedTexture {
    pub fn new(texture: Texture, spec: TextureSpec) -> Self {
        let view = texture.create_view(&TextureViewDescriptor::default());
        Self {
            texture,
            view,
            spec,
            last_used: std::time::Instant::now(),
            is_in_use: false,
        }
    }

    pub fn mark_used(&mut self) {
        self.last_used = std::time::Instant::now();
        self.is_in_use = true;
    }

    pub fn mark_unused(&mut self) {
        self.is_in_use = false;
    }
}

/// テクスチャ管理システム
pub struct TextureManager {
    /// アクティブなテクスチャ（レイヤーID -> テクスチャID）
    layer_textures: HashMap<String, String>,
    /// 管理対象のテクスチャ（テクスチャID -> テクスチャ）
    textures: HashMap<String, ManagedTexture>,
    /// テクスチャプール（仕様 -> 利用可能なテクスチャIDキュー）
    texture_pool: HashMap<TextureSpec, VecDeque<String>>,
    /// メモリ使用量監視
    current_memory_usage: u64,
    /// メモリ使用量上限（バイト）- デフォルト2GB
    memory_limit: u64,
    /// 次のテクスチャID
    next_texture_id: u64,
}

impl TextureManager {
    /// 新しいTextureManagerを作成
    pub fn new() -> Self {
        info!("[TextureManager] 新しいインスタンスを作成");
        Self {
            layer_textures: HashMap::new(),
            textures: HashMap::new(),
            texture_pool: HashMap::new(),
            current_memory_usage: 0,
            memory_limit: 2 * 1024 * 1024 * 1024, // 2GB
            next_texture_id: 1,
        }
    }

    /// メモリ使用量上限を設定
    pub fn set_memory_limit(&mut self, limit_bytes: u64) {
        debug!("[TextureManager] メモリ使用量上限を設定: {} bytes", limit_bytes);
        self.memory_limit = limit_bytes;
    }

    /// レイヤー用テクスチャを作成または取得
    pub fn create_layer_texture(
        &mut self,
        device: &Device,
        layer_id: &str,
        width: u32,
        height: u32,
    ) -> Result<&ManagedTexture, TextureError> {
        debug!("[TextureManager] レイヤーテクスチャ作成: {} ({}x{})", layer_id, width, height);

        // 寸法の検証（最大4K解像度をサポート）
        if width == 0 || height == 0 || width > 3840 || height > 2160 {
            return Err(TextureError::InvalidDimensions(width, height));
        }

        let spec = TextureSpec::layer_texture(width, height);
        
        // 既存のレイヤーテクスチャがある場合は解放
        if let Some(old_texture_id) = self.layer_textures.get(layer_id).cloned() {
            self.release_texture(&old_texture_id);
        }

        // プールから再利用可能なテクスチャを探す
        let texture_id = if let Some(reused_id) = self.get_texture_from_pool(&spec) {
            debug!("[TextureManager] プールからテクスチャを再利用: {}", reused_id);
            reused_id
        } else {
            // 新しいテクスチャを作成
            let texture_id = self.generate_texture_id();
            self.create_new_texture(device, &texture_id, &spec)?;
            texture_id
        };

        // レイヤーにテクスチャを関連付け
        self.layer_textures.insert(layer_id.to_string(), texture_id.clone());
        
        // テクスチャを使用中にマーク
        if let Some(managed_texture) = self.textures.get_mut(&texture_id) {
            managed_texture.mark_used();
            info!("[TextureManager] レイヤーテクスチャ作成完了: {}", layer_id);
            Ok(managed_texture)
        } else {
            Err(TextureError::TextureNotFound(texture_id))
        }
    }

    /// テクスチャからピクセルデータを取得
    pub async fn get_texture_data(
        &self,
        device: &Device,
        queue: &Queue,
        layer_id: &str,
    ) -> Result<Vec<u8>, TextureError> {
        debug!("[TextureManager] テクスチャデータ取得開始: {}", layer_id);

        let texture_id = self.layer_textures.get(layer_id)
            .ok_or_else(|| TextureError::TextureNotFound(layer_id.to_string()))?;

        let managed_texture = self.textures.get(texture_id)
            .ok_or_else(|| TextureError::TextureNotFound(texture_id.clone()))?;

        // バッファサイズの計算（アライメント考慮）
        let bytes_per_pixel = 4; // RGBA8
        let unpadded_bytes_per_row = managed_texture.spec.width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = (unpadded_bytes_per_row + align - 1) / align * align;
        let buffer_size = (padded_bytes_per_row * managed_texture.spec.height) as u64;

        // 読み取り用バッファを作成
        let output_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Texture Read Buffer"),
            size: buffer_size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // テクスチャからバッファにコピー
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Texture Copy Encoder"),
        });

        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                texture: &managed_texture.texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: &output_buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(managed_texture.spec.height),
                },
            },
            Extent3d {
                width: managed_texture.spec.width,
                height: managed_texture.spec.height,
                depth_or_array_layers: 1,
            },
        );

        queue.submit(std::iter::once(encoder.finish()));

        // バッファを読み取り
        let buffer_slice = output_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();
        buffer_slice.map_async(MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        let _ = device.poll(wgpu::MaintainBase::Wait);

        receiver.await
            .map_err(|_| TextureError::BufferReadFailed("バッファマップ待機に失敗".to_string()))?
            .map_err(|e| TextureError::BufferReadFailed(format!("バッファマップに失敗: {:?}", e)))?;

        let data = buffer_slice.get_mapped_range();
        let result = data.to_vec();
        
        drop(data);
        output_buffer.unmap();

        info!("[TextureManager] テクスチャデータ取得完了: {} ({} bytes)", layer_id, result.len());
        Ok(result)
    }

    /// テクスチャサイズを変更
    pub fn resize_texture(
        &mut self,
        device: &Device,
        layer_id: &str,
        width: u32,
        height: u32,
    ) -> Result<&ManagedTexture, TextureError> {
        debug!("[TextureManager] テクスチャリサイズ: {} ({}x{})", layer_id, width, height);

        // 新しいテクスチャを作成（内部的にはcreate_layer_textureと同じ）
        self.create_layer_texture(device, layer_id, width, height)
    }

    /// テクスチャをクリア（透明色で塗りつぶし）
    pub fn clear_texture(
        &mut self,
        device: &Device,
        queue: &Queue,
        layer_id: &str,
        clear_color: Option<Color>,
    ) -> Result<(), TextureError> {
        debug!("[TextureManager] テクスチャクリア: {}", layer_id);

        let texture_id = self.layer_textures.get(layer_id)
            .ok_or_else(|| TextureError::TextureNotFound(layer_id.to_string()))?;

        let managed_texture = self.textures.get_mut(texture_id)
            .ok_or_else(|| TextureError::TextureNotFound(texture_id.clone()))?;

        // クリア色の設定（デフォルトは透明）
        let color = clear_color.unwrap_or(Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        });

        // レンダパスでクリア
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Texture Clear Encoder"),
        });

        {
            let _render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Texture Clear Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &managed_texture.view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(color),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        queue.submit(std::iter::once(encoder.finish()));
        managed_texture.mark_used();

        info!("[TextureManager] テクスチャクリア完了: {}", layer_id);
        Ok(())
    }

    /// レイヤーテクスチャを取得
    pub fn get_layer_texture(&self, layer_id: &str) -> Option<&ManagedTexture> {
        let texture_id = self.layer_textures.get(layer_id)?;
        self.textures.get(texture_id)
    }

    /// レイヤーテクスチャを削除
    pub fn remove_layer_texture(&mut self, layer_id: &str) -> bool {
        if let Some(texture_id) = self.layer_textures.remove(layer_id) {
            self.release_texture(&texture_id);
            info!("[TextureManager] レイヤーテクスチャ削除: {}", layer_id);
            true
        } else {
            false
        }
    }

    /// 未使用のテクスチャをクリーンアップ
    pub fn cleanup_unused_textures(&mut self) {
        let cleanup_threshold = std::time::Duration::from_secs(300); // 5分
        let now = std::time::Instant::now();
        
        let mut textures_to_remove = Vec::new();
        
        for (texture_id, managed_texture) in &self.textures {
            if !managed_texture.is_in_use && now.duration_since(managed_texture.last_used) > cleanup_threshold {
                textures_to_remove.push(texture_id.clone());
            }
        }

        for texture_id in textures_to_remove {
            self.remove_texture_completely(&texture_id);
        }

        if !self.textures.is_empty() {
            debug!("[TextureManager] クリーンアップ完了: {} テクスチャが残存", self.textures.len());
        }
    }

    /// 現在のメモリ使用量を取得
    pub fn get_memory_usage(&self) -> u64 {
        self.current_memory_usage
    }

    /// メモリ使用量統計を取得
    pub fn get_memory_stats(&self) -> (u64, u64, usize, usize) {
        let active_textures = self.layer_textures.len();
        let total_textures = self.textures.len();
        (self.current_memory_usage, self.memory_limit, active_textures, total_textures)
    }

    // プライベートメソッド

    fn generate_texture_id(&mut self) -> String {
        let id = format!("tex_{}", self.next_texture_id);
        self.next_texture_id += 1;
        id
    }

    fn create_new_texture(
        &mut self,
        device: &Device,
        texture_id: &str,
        spec: &TextureSpec,
    ) -> Result<(), TextureError> {
        // メモリ使用量チェック
        let texture_memory = spec.memory_size();
        if self.current_memory_usage + texture_memory > self.memory_limit {
            // メモリ不足の場合、古いテクスチャをクリーンアップ
            self.force_cleanup_memory(texture_memory)?;
        }

        let texture = device.create_texture(&TextureDescriptor {
            label: Some(&format!("Managed Texture {}", texture_id)),
            size: Extent3d {
                width: spec.width,
                height: spec.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: spec.format,
            usage: spec.usage,
            view_formats: &[],
        });

        let managed_texture = ManagedTexture::new(texture, spec.clone());
        self.textures.insert(texture_id.to_string(), managed_texture);
        self.current_memory_usage += texture_memory;

        debug!("[TextureManager] 新しいテクスチャ作成: {} ({} bytes)", texture_id, texture_memory);
        Ok(())
    }

    fn get_texture_from_pool(&mut self, spec: &TextureSpec) -> Option<String> {
        let pool = self.texture_pool.get_mut(spec)?;
        pool.pop_front()
    }

    fn release_texture(&mut self, texture_id: &str) {
        if let Some(mut managed_texture) = self.textures.remove(texture_id) {
            managed_texture.mark_unused();
            
            // プールに戻す
            let pool = self.texture_pool.entry(managed_texture.spec.clone()).or_default();
            pool.push_back(texture_id.to_string());
            self.textures.insert(texture_id.to_string(), managed_texture);

            debug!("[TextureManager] テクスチャをプールに戻しました: {}", texture_id);
        }
    }

    fn remove_texture_completely(&mut self, texture_id: &str) {
        if let Some(managed_texture) = self.textures.remove(texture_id) {
            self.current_memory_usage -= managed_texture.spec.memory_size();
            
            // プールからも削除
            if let Some(pool) = self.texture_pool.get_mut(&managed_texture.spec) {
                pool.retain(|id| id != texture_id);
            }

            debug!("[TextureManager] テクスチャを完全削除: {}", texture_id);
        }
    }

    fn force_cleanup_memory(&mut self, required_memory: u64) -> Result<(), TextureError> {
        let initial_usage = self.current_memory_usage;
        
        // 使用されていないテクスチャを削除
        let mut textures_to_remove = Vec::new();
        for (texture_id, managed_texture) in &self.textures {
            if !managed_texture.is_in_use {
                textures_to_remove.push(texture_id.clone());
            }
        }

        // 最後に使用された時間でソート（古い順）
        textures_to_remove.sort_by(|a, b| {
            let time_a = self.textures.get(a).unwrap().last_used;
            let time_b = self.textures.get(b).unwrap().last_used;
            time_a.cmp(&time_b)
        });

        for texture_id in textures_to_remove {
            self.remove_texture_completely(&texture_id);
            if self.current_memory_usage + required_memory <= self.memory_limit {
                break;
            }
        }

        if self.current_memory_usage + required_memory > self.memory_limit {
            error!("[TextureManager] メモリクリーンアップ後もメモリ不足: 必要{} / 利用可能{}", 
                required_memory, self.memory_limit - self.current_memory_usage);
            return Err(TextureError::MemoryLimitExceeded(required_memory));
        }

        let freed_memory = initial_usage - self.current_memory_usage;
        info!("[TextureManager] 強制メモリクリーンアップ完了: {} bytes解放", freed_memory);
        Ok(())
    }
}

impl Drop for TextureManager {
    fn drop(&mut self) {
        info!("[TextureManager] テクスチャマネージャーを解放: {} テクスチャ, {} bytes", 
            self.textures.len(), self.current_memory_usage);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_device() -> (Device, Queue) {
        pollster::block_on(async {
            let instance = Instance::new(&InstanceDescriptor {
                backends: Backends::all(),
                flags: InstanceFlags::default(),
                ..Default::default()
            });

            let adapter = instance
                .request_adapter(&RequestAdapterOptions {
                    power_preference: PowerPreference::HighPerformance,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await
                .expect("Failed to find an appropriate adapter");

            adapter
                .request_device(
                    &DeviceDescriptor {
                        label: Some("Test Device"),
                        required_features: Features::empty(),
                        required_limits: Limits::default(),
                        ..Default::default()
                    },
                )
                .await
                .expect("Failed to create device")
        })
    }

    #[test]
    fn test_texture_spec_creation() {
        let spec = TextureSpec::layer_texture(1920, 1080);
        assert_eq!(spec.width, 1920);
        assert_eq!(spec.height, 1080);
        assert_eq!(spec.format, TextureFormat::Rgba8UnormSrgb);
        assert!(spec.usage.contains(TextureUsages::RENDER_ATTACHMENT));
        assert!(spec.usage.contains(TextureUsages::COPY_SRC));
        assert!(spec.usage.contains(TextureUsages::COPY_DST));
    }

    #[test]
    fn test_texture_spec_memory_size() {
        let spec = TextureSpec::layer_texture(1920, 1080);
        let expected_size = 1920 * 1080 * 4; // RGBA8 = 4 bytes per pixel
        assert_eq!(spec.memory_size(), expected_size as u64);
    }

    #[test]
    fn test_texture_manager_creation() {
        let manager = TextureManager::new();
        assert_eq!(manager.get_memory_usage(), 0);
        let (current, limit, active, total) = manager.get_memory_stats();
        assert_eq!(current, 0);
        assert!(limit > 0);
        assert_eq!(active, 0);
        assert_eq!(total, 0);
    }

    #[tokio::test]
    async fn test_create_layer_texture() {
        let (device, _queue) = create_test_device();
        let mut manager = TextureManager::new();

        let result = manager.create_layer_texture(&device, "layer1", 512, 512);
        assert!(result.is_ok());

        let texture = manager.get_layer_texture("layer1");
        assert!(texture.is_some());

        let texture = texture.unwrap();
        assert_eq!(texture.spec.width, 512);
        assert_eq!(texture.spec.height, 512);

        let (memory_usage, _, active_textures, total_textures) = manager.get_memory_stats();
        assert!(memory_usage > 0);
        assert_eq!(active_textures, 1);
        assert_eq!(total_textures, 1);
    }

    #[tokio::test]
    async fn test_invalid_dimensions() {
        let (device, _queue) = create_test_device();
        let mut manager = TextureManager::new();

        // 無効な寸法でテクスチャ作成を試行
        let result = manager.create_layer_texture(&device, "invalid", 0, 256);
        assert!(result.is_err());

        let result = manager.create_layer_texture(&device, "invalid", 256, 0);
        assert!(result.is_err());

        // 4Kを超える寸法
        let result = manager.create_layer_texture(&device, "invalid", 5000, 256);
        assert!(result.is_err());
    }

    #[test]
    fn test_texture_error_display() {
        let error = TextureError::InvalidDimensions(0, 256);
        let error_string = format!("{}", error);
        assert!(error_string.contains("無効な寸法"));
        assert!(error_string.contains("0x256"));

        let error = TextureError::TextureNotFound("test_texture".to_string());
        let error_string = format!("{}", error);
        assert!(error_string.contains("テクスチャが見つかりません"));
        assert!(error_string.contains("test_texture"));
    }
}