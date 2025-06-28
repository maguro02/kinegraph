
use wgpu::*;
use log::{info, error, debug};

pub struct DrawingEngine {
    instance: Instance,
    pub surface: Option<Surface<'static>>,
    pub adapter: Option<Adapter>,
    pub device: Option<Device>,
    pub queue: Option<Queue>,
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
        self.device = Some(device);
        self.queue = Some(queue);
        
        info!("[DrawingEngine] 初期化正常完了");
        Ok(())
    }
}
