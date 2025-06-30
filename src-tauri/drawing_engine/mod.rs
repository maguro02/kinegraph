
use wgpu::*;
use log::{info, error, debug};

pub mod renderer;
pub mod texture;
pub mod pipeline;

#[cfg(test)]
mod pipeline_test;
pub use renderer::{OffscreenRenderer, OffscreenRenderError};
pub use texture::{TextureManager, TextureError, TextureSpec, ManagedTexture};
pub use pipeline::{BasicDrawPipeline, PipelineError, DrawStroke, Vertex2D};

pub struct DrawingEngine {
    instance: Instance,
    pub surface: Option<Surface<'static>>,
    pub adapter: Option<Adapter>,
    pub device: Option<Device>,
    pub queue: Option<Queue>,
    pub texture_manager: Option<TextureManager>,
    pub draw_pipeline: Option<BasicDrawPipeline>,
}

impl DrawingEngine {
    pub fn new() -> Self {
        debug!("[DrawingEngine] 新しい DrawingEngine インスタンス作成開始");
        
        debug!("[DrawingEngine] wgpu Instance 作成中...");
        let instance = Instance::new(&InstanceDescriptor {
            backends: Backends::all(),
            flags: InstanceFlags::default(),
            ..Default::default()
        });
        debug!("[DrawingEngine] wgpu Instance 作成完了");
        
        let engine = Self {
            instance,
            surface: None,
            adapter: None,
            device: None,
            queue: None,
            texture_manager: None,
            draw_pipeline: None,
        };
        
        info!("[DrawingEngine] DrawingEngine インスタンス作成完了");
        engine
    }

    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("[DrawingEngine] 初期化開始");
        
        debug!("[DrawingEngine] 利用可能なアダプターを検索中...");
        let adapter = self
            .instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: self.surface.as_ref(),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| format!("Failed to find an appropriate adapter: {:?}", e))?;
            
        info!("[DrawingEngine] アダプター検索成功");
        debug!("[DrawingEngine] アダプター情報: {:?}", adapter.get_info());

        debug!("[DrawingEngine] デバイスとキューをリクエスト中...");
        let device_result = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("Kinegraph Drawing Device"),
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
                    ..Default::default()
                },
            )
            .await;
            
        let (device, queue) = match device_result {
            Ok((device, queue)) => {
                info!("[DrawingEngine] デバイスとキューの作成成功");
                debug!("[DrawingEngine] デバイス作成完了");
                (device, queue)
            },
            Err(e) => {
                error!("[DrawingEngine] デバイス作成失敗: {}", e);
                return Err(Box::new(e));
            }
        };

        debug!("[DrawingEngine] DrawingEngine 状態を更新中...");
        self.adapter = Some(adapter);
        
        // 描画パイプラインを初期化（deviceを使用する前に）
        debug!("[DrawingEngine] BasicDrawPipeline 初期化中...");
        let pipeline = BasicDrawPipeline::new(&device, TextureFormat::Rgba8UnormSrgb)
            .map_err(|e| format!("描画パイプライン初期化失敗: {}", e))?;
        self.draw_pipeline = Some(pipeline);
        
        // deviceとqueueを保存
        self.device = Some(device);
        self.queue = Some(queue);
        
        // TextureManagerを初期化
        debug!("[DrawingEngine] TextureManager 初期化中...");
        self.texture_manager = Some(TextureManager::new());
        
        info!("[DrawingEngine] 初期化正常完了");
        Ok(())
    }

    /// オフスクリーンレンダラーを作成
    pub fn create_offscreen_renderer(&self, width: u32, height: u32) -> Result<OffscreenRenderer, OffscreenRenderError> {
        debug!("[DrawingEngine] オフスクリーンレンダラー作成開始: {}x{}", width, height);
        
        let mut renderer = OffscreenRenderer::new(width, height)?;
        
        if let Some(device) = &self.device {
            renderer.initialize(device)?;
            info!("[DrawingEngine] オフスクリーンレンダラー作成完了");
            Ok(renderer)
        } else {
            error!("[DrawingEngine] Device が初期化されていません");
            Err(OffscreenRenderError::DeviceNotInitialized)
        }
    }

    /// オフスクリーンレンダリングを実行してピクセルデータを取得
    pub async fn render_offscreen(&self, renderer: &OffscreenRenderer) -> Result<Vec<u8>, OffscreenRenderError> {
        debug!("[DrawingEngine] オフスクリーンレンダリング開始");
        
        let device = self.device.as_ref()
            .ok_or(OffscreenRenderError::DeviceNotInitialized)?;
        let queue = self.queue.as_ref()
            .ok_or(OffscreenRenderError::QueueNotInitialized)?;

        let result = renderer.render_to_buffer(device, queue).await?;
        info!("[DrawingEngine] オフスクリーンレンダリング完了: {} バイト", result.len());
        Ok(result)
    }

    /// TextureManagerの参照を取得
    pub fn texture_manager(&self) -> Option<&TextureManager> {
        self.texture_manager.as_ref()
    }

    /// TextureManagerの可変参照を取得
    pub fn texture_manager_mut(&mut self) -> Option<&mut TextureManager> {
        self.texture_manager.as_mut()
    }

    /// レイヤー用テクスチャを作成
    pub fn create_layer_texture(&mut self, layer_id: &str, width: u32, height: u32) -> Result<(), TextureError> {
        debug!("[DrawingEngine] レイヤーテクスチャ作成: {} ({}x{})", layer_id, width, height);
        
        let device = self.device.as_ref()
            .ok_or(TextureError::DeviceNotInitialized)?;
        let texture_manager = self.texture_manager.as_mut()
            .ok_or(TextureError::DeviceNotInitialized)?;

        texture_manager.create_layer_texture(device, layer_id, width, height)?;
        Ok(())
    }

    /// レイヤーテクスチャのピクセルデータを取得
    pub async fn get_layer_texture_data(&self, layer_id: &str) -> Result<Vec<u8>, TextureError> {
        debug!("[DrawingEngine] レイヤーテクスチャデータ取得: {}", layer_id);
        
        let device = self.device.as_ref()
            .ok_or(TextureError::DeviceNotInitialized)?;
        let queue = self.queue.as_ref()
            .ok_or(TextureError::DeviceNotInitialized)?;
        let texture_manager = self.texture_manager.as_ref()
            .ok_or(TextureError::DeviceNotInitialized)?;

        texture_manager.get_texture_data(device, queue, layer_id).await
    }

    /// レイヤーテクスチャをクリア
    pub fn clear_layer_texture(&mut self, layer_id: &str, clear_color: Option<wgpu::Color>) -> Result<(), TextureError> {
        debug!("[DrawingEngine] レイヤーテクスチャクリア: {}", layer_id);
        
        let device = self.device.as_ref()
            .ok_or(TextureError::DeviceNotInitialized)?;
        let queue = self.queue.as_ref()
            .ok_or(TextureError::DeviceNotInitialized)?;
        let texture_manager = self.texture_manager.as_mut()
            .ok_or(TextureError::DeviceNotInitialized)?;

        texture_manager.clear_texture(device, queue, layer_id, clear_color)
    }

    /// レイヤーテクスチャを削除
    pub fn remove_layer_texture(&mut self, layer_id: &str) -> bool {
        if let Some(texture_manager) = self.texture_manager.as_mut() {
            texture_manager.remove_layer_texture(layer_id)
        } else {
            false
        }
    }

    /// 未使用テクスチャのクリーンアップ
    pub fn cleanup_unused_textures(&mut self) {
        if let Some(texture_manager) = self.texture_manager.as_mut() {
            texture_manager.cleanup_unused_textures();
        }
    }

    /// メモリ使用量統計を取得
    pub fn get_texture_memory_stats(&self) -> Option<(u64, u64, usize, usize)> {
        self.texture_manager.as_ref().map(|tm| tm.get_memory_stats())
    }

    /// レイヤーテクスチャに線を描画
    pub fn draw_line_to_layer(
        &self,
        layer_id: &str,
        start: (f32, f32),
        end: (f32, f32),
        color: [f32; 4],
        width: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        debug!("[DrawingEngine] レイヤーに線描画: {} {:?} -> {:?}", layer_id, start, end);
        
        let device = self.device.as_ref()
            .ok_or("Device が初期化されていません")?;
        let queue = self.queue.as_ref()
            .ok_or("Queue が初期化されていません")?;
        let texture_manager = self.texture_manager.as_ref()
            .ok_or("TextureManager が初期化されていません")?;
        let pipeline = self.draw_pipeline.as_ref()
            .ok_or("DrawPipeline が初期化されていません")?;

        // レイヤーテクスチャを取得
        let managed_texture = texture_manager.get_layer_texture(layer_id)
            .ok_or(format!("レイヤーテクスチャが見つかりません: {}", layer_id))?;

        // コマンドエンコーダーを作成
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Draw Line Encoder"),
        });

        // 線を描画
        pipeline.draw_line(
            device,
            queue,
            &mut encoder,
            &managed_texture.view,
            start,
            end,
            color,
            width,
        )?;

        // コマンドを送信
        queue.submit(std::iter::once(encoder.finish()));

        info!("[DrawingEngine] レイヤーに線描画完了: {}", layer_id);
        Ok(())
    }

    /// レイヤーテクスチャにストロークを描画
    pub fn draw_stroke_to_layer(
        &self,
        layer_id: &str,
        stroke: &DrawStroke,
    ) -> Result<(), Box<dyn std::error::Error>> {
        debug!("[DrawingEngine] レイヤーにストローク描画: {} ({} 点)", layer_id, stroke.points.len());
        
        let device = self.device.as_ref()
            .ok_or("Device が初期化されていません")?;
        let queue = self.queue.as_ref()
            .ok_or("Queue が初期化されていません")?;
        let texture_manager = self.texture_manager.as_ref()
            .ok_or("TextureManager が初期化されていません")?;
        let pipeline = self.draw_pipeline.as_ref()
            .ok_or("DrawPipeline が初期化されていません")?;

        // レイヤーテクスチャを取得
        let managed_texture = texture_manager.get_layer_texture(layer_id)
            .ok_or(format!("レイヤーテクスチャが見つかりません: {}", layer_id))?;

        // コマンドエンコーダーを作成
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Draw Stroke Encoder"),
        });

        // ストロークを描画
        pipeline.draw_stroke(
            device,
            queue,
            &mut encoder,
            &managed_texture.view,
            stroke,
        )?;

        // コマンドを送信
        queue.submit(std::iter::once(encoder.finish()));

        info!("[DrawingEngine] レイヤーにストローク描画完了: {}", layer_id);
        Ok(())
    }

    /// スクリーン座標を正規化座標に変換（描画用）
    pub fn screen_to_normalized(&self, screen_pos: (f32, f32), screen_size: (u32, u32)) -> (f32, f32) {
        BasicDrawPipeline::screen_to_normalized(screen_pos, screen_size)
    }

    /// 正規化座標をスクリーン座標に変換
    pub fn normalized_to_screen(&self, norm_pos: (f32, f32), screen_size: (u32, u32)) -> (f32, f32) {
        BasicDrawPipeline::normalized_to_screen(norm_pos, screen_size)
    }
}
