use wgpu::*;
use log::{info, debug, warn};
use std::error::Error;
use std::fmt;

/// オフスクリーンレンダリングのエラー型
#[derive(Debug)]
pub enum OffscreenRenderError {
    DeviceNotInitialized,
    QueueNotInitialized,
    TextureCreationFailed(String),
    BufferCreationFailed(String),
    RenderingFailed(String),
    BufferReadFailed(String),
    InvalidDimensions(u32, u32),
}

impl fmt::Display for OffscreenRenderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OffscreenRenderError::DeviceNotInitialized => {
                write!(f, "wgpu Device が初期化されていません")
            }
            OffscreenRenderError::QueueNotInitialized => {
                write!(f, "wgpu Queue が初期化されていません")
            }
            OffscreenRenderError::TextureCreationFailed(msg) => {
                write!(f, "テクスチャ作成に失敗しました: {}", msg)
            }
            OffscreenRenderError::BufferCreationFailed(msg) => {
                write!(f, "バッファ作成に失敗しました: {}", msg)
            }
            OffscreenRenderError::RenderingFailed(msg) => {
                write!(f, "レンダリングに失敗しました: {}", msg)
            }
            OffscreenRenderError::BufferReadFailed(msg) => {
                write!(f, "バッファ読み取りに失敗しました: {}", msg)
            }
            OffscreenRenderError::InvalidDimensions(width, height) => {
                write!(f, "無効な寸法です: {}x{}", width, height)
            }
        }
    }
}

impl Error for OffscreenRenderError {}

/// オフスクリーンレンダリング用構造体
pub struct OffscreenRenderer {
    pub width: u32,
    pub height: u32,
    pub padded_bytes_per_row: u32,
    pub texture: Option<Texture>,
    pub render_texture_view: Option<TextureView>,
    pub output_buffer: Option<Buffer>,
}

impl OffscreenRenderer {
    /// 新しいOffscreenRendererインスタンスを作成
    pub fn new(width: u32, height: u32) -> Result<Self, OffscreenRenderError> {
        // 寸法の検証（最大4K解像度をサポート）
        if width == 0 || height == 0 || width > 3840 || height > 2160 {
            return Err(OffscreenRenderError::InvalidDimensions(width, height));
        }

        debug!("[OffscreenRenderer] 新しいインスタンス作成: {}x{}", width, height);
        
        // バイト数の計算（アライメント考慮）
        let bytes_per_pixel = 4; // RGBA8 = 4バイト/ピクセル
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = (unpadded_bytes_per_row + align - 1) / align * align;
        
        Ok(Self {
            width,
            height,
            padded_bytes_per_row,
            texture: None,
            render_texture_view: None,
            output_buffer: None,
        })
    }

    /// wgpu Device を使用してテクスチャとバッファを初期化
    pub fn initialize(&mut self, device: &Device) -> Result<(), OffscreenRenderError> {
        info!("[OffscreenRenderer] 初期化開始: {}x{}", self.width, self.height);

        // オフスクリーンレンダリング用テクスチャを作成
        debug!("[OffscreenRenderer] レンダリングテクスチャ作成中...");
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Offscreen Render Texture"),
            size: Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        // テクスチャビューを作成
        let render_texture_view = texture.create_view(&TextureViewDescriptor::default());
        debug!("[OffscreenRenderer] テクスチャビュー作成完了");

        // CPUアクセス用のバッファを作成（アライメント考慮）
        debug!("[OffscreenRenderer] 出力バッファ作成中...");
        let buffer_size = (self.padded_bytes_per_row * self.height) as u64;
        let output_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Offscreen Output Buffer"),
            size: buffer_size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // 構造体に保存
        self.texture = Some(texture);
        self.render_texture_view = Some(render_texture_view);
        self.output_buffer = Some(output_buffer);

        info!("[OffscreenRenderer] 初期化完了");
        Ok(())
    }

    /// 空のキャンバスをレンダリングしてピクセルデータを返す
    pub async fn render_to_buffer(
        &self,
        device: &Device,
        queue: &Queue,
    ) -> Result<Vec<u8>, OffscreenRenderError> {
        debug!("[OffscreenRenderer] render_to_buffer 開始");

        // 必要なリソースの存在確認
        let texture = self.texture.as_ref()
            .ok_or(OffscreenRenderError::TextureCreationFailed("テクスチャが初期化されていません".to_string()))?;
        let render_texture_view = self.render_texture_view.as_ref()
            .ok_or(OffscreenRenderError::TextureCreationFailed("テクスチャビューが初期化されていません".to_string()))?;
        let output_buffer = self.output_buffer.as_ref()
            .ok_or(OffscreenRenderError::BufferCreationFailed("出力バッファが初期化されていません".to_string()))?;

        // コマンドエンコーダーを作成
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Offscreen Render Encoder"),
        });

        // レンダパスを開始（空のキャンバスを白色でクリア）
        {
            let _render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Offscreen Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: render_texture_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 1.0, // 白色の背景
                            g: 1.0,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // レンダパスはここで自動的に終了
        }

        // テクスチャからバッファにコピー
        debug!("[OffscreenRenderer] テクスチャをバッファにコピー中...");
        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: output_buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padded_bytes_per_row), // アライメント考慮
                    rows_per_image: Some(self.height),
                },
            },
            Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        // コマンドを送信
        queue.submit(std::iter::once(encoder.finish()));

        // バッファを読み取り
        debug!("[OffscreenRenderer] バッファからピクセルデータを読み取り中...");
        let buffer_slice = output_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();
        buffer_slice.map_async(MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        // デバイスをポーリングしてマップ操作を完了
        let _ = device.poll(wgpu::MaintainBase::Wait);

        // 結果を待機
        receiver.await
            .map_err(|_| OffscreenRenderError::BufferReadFailed("バッファマップ待機に失敗".to_string()))?
            .map_err(|e| OffscreenRenderError::BufferReadFailed(format!("バッファマップに失敗: {:?}", e)))?;

        // マップされたデータを取得
        let data = buffer_slice.get_mapped_range();
        let result = data.to_vec();
        
        // バッファをアンマップ
        drop(data);
        output_buffer.unmap();

        info!("[OffscreenRenderer] render_to_buffer 完了: {} バイト", result.len());
        Ok(result)
    }

    /// 現在の寸法を取得
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// 新しい寸法でリサイズ（再初期化が必要）
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), OffscreenRenderError> {
        if width == 0 || height == 0 || width > 3840 || height > 2160 {
            return Err(OffscreenRenderError::InvalidDimensions(width, height));
        }

        debug!("[OffscreenRenderer] リサイズ: {}x{} -> {}x{}", self.width, self.height, width, height);
        self.width = width;
        self.height = height;
        
        // バイト数を再計算
        let bytes_per_pixel = 4; // RGBA8 = 4バイト/ピクセル
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        self.padded_bytes_per_row = (unpadded_bytes_per_row + align - 1) / align * align;
        
        // リソースをクリア（再初期化が必要）
        self.texture = None;
        self.render_texture_view = None;
        self.output_buffer = None;
        
        warn!("[OffscreenRenderer] リサイズ後はinitialize()を再度呼び出してください");
        Ok(())
    }
}

impl Drop for OffscreenRenderer {
    fn drop(&mut self) {
        debug!("[OffscreenRenderer] リソースを解放中");
    }
}