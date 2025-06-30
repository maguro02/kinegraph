use wgpu::*;
use log::{info, debug};
use std::error::Error;
use std::fmt;

/// 描画パイプラインのエラー型
#[derive(Debug)]
pub enum PipelineError {
    PipelineCreationFailed(String),
    ShaderCompilationFailed(String),
    BufferCreationFailed(String),
    RenderingFailed(String),
    InvalidVertexData(String),
    DeviceNotAvailable,
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PipelineError::PipelineCreationFailed(msg) => {
                write!(f, "パイプライン作成に失敗しました: {}", msg)
            }
            PipelineError::ShaderCompilationFailed(msg) => {
                write!(f, "シェーダーコンパイルに失敗しました: {}", msg)
            }
            PipelineError::BufferCreationFailed(msg) => {
                write!(f, "バッファ作成に失敗しました: {}", msg)
            }
            PipelineError::RenderingFailed(msg) => {
                write!(f, "描画に失敗しました: {}", msg)
            }
            PipelineError::InvalidVertexData(msg) => {
                write!(f, "無効な頂点データです: {}", msg)
            }
            PipelineError::DeviceNotAvailable => {
                write!(f, "wgpu Device が利用できません")
            }
        }
    }
}

impl Error for PipelineError {}

/// 2D描画用の頂点データ
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex2D {
    /// 正規化座標 (-1.0 ～ 1.0)
    pub position: [f32; 2],
    /// RGBA色 (0.0 ～ 1.0)
    pub color: [f32; 4],
    /// 線の幅（筆圧対応の準備）
    pub line_width: f32,
}

impl Vertex2D {
    /// 新しい頂点を作成
    pub fn new(x: f32, y: f32, color: [f32; 4], line_width: f32) -> Self {
        Self {
            position: [x, y],
            color,
            line_width,
        }
    }

    /// 頂点レイアウトを取得
    pub fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex2D>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                // Position
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x2,
                },
                // Color
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x4,
                },
                // Line Width
                VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as BufferAddress,
                    shader_location: 2,
                    format: VertexFormat::Float32,
                },
            ],
        }
    }
}

/// 描画ストローク（連続する点のデータ）
#[derive(Debug, Clone)]
pub struct DrawStroke {
    /// ストロークの点
    pub points: Vec<Vertex2D>,
    /// ストロークの色
    pub color: [f32; 4],
    /// 基本線の幅
    pub base_width: f32,
    /// 閉じたストロークかどうか
    pub is_closed: bool,
}

impl DrawStroke {
    /// 新しいストロークを作成
    pub fn new(color: [f32; 4], base_width: f32) -> Self {
        Self {
            points: Vec::new(),
            color,
            base_width,
            is_closed: false,
        }
    }

    /// 点を追加
    pub fn add_point(&mut self, x: f32, y: f32, pressure: f32) {
        let width = self.base_width * pressure.clamp(0.1, 2.0);
        self.points.push(Vertex2D::new(x, y, self.color, width));
    }

    /// ストロークを閉じる
    pub fn close(&mut self) {
        self.is_closed = true;
    }

    /// 三角形データに変換（線分の描画用）
    pub fn to_triangles(&self) -> Vec<Vertex2D> {
        if self.points.len() < 2 {
            return Vec::new();
        }

        let mut triangles = Vec::new();
        
        for i in 0..self.points.len() - 1 {
            let p1 = &self.points[i];
            let p2 = &self.points[i + 1];
            
            // 線分の方向ベクトルを計算
            let dx = p2.position[0] - p1.position[0];
            let dy = p2.position[1] - p1.position[1];
            let length = (dx * dx + dy * dy).sqrt();
            
            if length < 1e-6 {
                continue; // 長さがゼロの線分はスキップ
            }
            
            // 法線ベクトル（線分に垂直）
            let nx = -dy / length;
            let ny = dx / length;
            
            // 線の幅を考慮した4つの頂点を計算
            let half_width1 = p1.line_width * 0.001; // 正規化座標での幅調整
            let half_width2 = p2.line_width * 0.001;
            
            let v1 = Vertex2D::new(
                p1.position[0] + nx * half_width1,
                p1.position[1] + ny * half_width1,
                p1.color,
                p1.line_width,
            );
            let v2 = Vertex2D::new(
                p1.position[0] - nx * half_width1,
                p1.position[1] - ny * half_width1,
                p1.color,
                p1.line_width,
            );
            let v3 = Vertex2D::new(
                p2.position[0] + nx * half_width2,
                p2.position[1] + ny * half_width2,
                p2.color,
                p2.line_width,
            );
            let v4 = Vertex2D::new(
                p2.position[0] - nx * half_width2,
                p2.position[1] - ny * half_width2,
                p2.color,
                p2.line_width,
            );
            
            // 2つの三角形を追加（四角形を構成）
            triangles.extend_from_slice(&[v1, v2, v3, v2, v4, v3]);
        }
        
        triangles
    }
}

/// 基本描画パイプライン
pub struct BasicDrawPipeline {
    /// 描画パイプライン
    render_pipeline: RenderPipeline,
    /// 頂点バッファ
    vertex_buffer: Buffer,
    /// 最大頂点数
    max_vertices: usize,
}

impl BasicDrawPipeline {
    /// 新しい描画パイプラインを作成
    pub fn new(device: &Device, format: TextureFormat) -> Result<Self, PipelineError> {
        info!("[BasicDrawPipeline] 新しいパイプライン作成開始");

        // 頂点シェーダー
        let vertex_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Vertex Shader"),
            source: ShaderSource::Wgsl(Self::vertex_shader_source().into()),
        });

        // フラグメントシェーダー
        let fragment_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Fragment Shader"), 
            source: ShaderSource::Wgsl(Self::fragment_shader_source().into()),
        });

        debug!("[BasicDrawPipeline] シェーダー作成完了");

        // パイプラインレイアウト
        let render_pipeline_layout =
            device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Draw Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        // レンダーパイプライン作成
        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Basic Draw Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &vertex_shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex2D::desc()],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &fragment_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        debug!("[BasicDrawPipeline] レンダーパイプライン作成完了");

        // 頂点バッファ作成（最大10000頂点）
        let max_vertices = 10000;
        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: (max_vertices * std::mem::size_of::<Vertex2D>()) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        info!("[BasicDrawPipeline] パイプライン作成完了: 最大{}頂点", max_vertices);

        Ok(Self {
            render_pipeline,
            vertex_buffer,
            max_vertices,
        })
    }

    /// 2点間の線を描画
    pub fn draw_line(
        &self,
        _device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        target_view: &TextureView,
        start: (f32, f32),
        end: (f32, f32),
        color: [f32; 4],
        width: f32,
    ) -> Result<(), PipelineError> {
        debug!("[BasicDrawPipeline] 線描画: {:?} -> {:?}", start, end);

        let mut stroke = DrawStroke::new(color, width);
        stroke.add_point(start.0, start.1, 1.0);
        stroke.add_point(end.0, end.1, 1.0);

        self.draw_stroke(_device, queue, encoder, target_view, &stroke)
    }

    /// ストローク（連続する点）を描画
    pub fn draw_stroke(
        &self,
        _device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        target_view: &TextureView,
        stroke: &DrawStroke,
    ) -> Result<(), PipelineError> {
        debug!("[BasicDrawPipeline] ストローク描画: {} 点", stroke.points.len());

        if stroke.points.is_empty() {
            return Ok(());
        }

        // 三角形データに変換
        let triangles = stroke.to_triangles();
        if triangles.is_empty() {
            return Ok(());
        }

        if triangles.len() > self.max_vertices {
            return Err(PipelineError::InvalidVertexData(
                format!("頂点数が上限を超えています: {} > {}", triangles.len(), self.max_vertices)
            ));
        }

        // 頂点データをバッファに書き込み
        let vertex_data = bytemuck::cast_slice(&triangles);
        queue.write_buffer(&self.vertex_buffer, 0, vertex_data);

        // レンダーパスを開始
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Draw Stroke Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load, // 既存の内容を保持
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // パイプラインを設定
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        // 描画
        render_pass.draw(0..triangles.len() as u32, 0..1);

        drop(render_pass);
        info!("[BasicDrawPipeline] ストローク描画完了: {} 三角形", triangles.len() / 3);
        Ok(())
    }

    /// 座標変換：スクリーン座標 -> 正規化座標
    pub fn screen_to_normalized(screen_pos: (f32, f32), screen_size: (u32, u32)) -> (f32, f32) {
        let x = (screen_pos.0 / screen_size.0 as f32) * 2.0 - 1.0;
        let y = 1.0 - (screen_pos.1 / screen_size.1 as f32) * 2.0; // Y軸反転
        (x, y)
    }

    /// 座標変換：正規化座標 -> スクリーン座標
    pub fn normalized_to_screen(norm_pos: (f32, f32), screen_size: (u32, u32)) -> (f32, f32) {
        let x = (norm_pos.0 + 1.0) * 0.5 * screen_size.0 as f32;
        let y = (1.0 - norm_pos.1) * 0.5 * screen_size.1 as f32; // Y軸反転
        (x, y)
    }

    /// 頂点シェーダーのソースコード（WGSL）
    fn vertex_shader_source() -> &'static str {
        r#"
        struct VertexInput {
            @location(0) position: vec2<f32>,
            @location(1) color: vec4<f32>,
            @location(2) line_width: f32,
        }

        struct VertexOutput {
            @builtin(position) clip_position: vec4<f32>,
            @location(0) color: vec4<f32>,
            @location(1) line_width: f32,
        }

        @vertex
        fn vs_main(model: VertexInput) -> VertexOutput {
            var out: VertexOutput;
            out.color = model.color;
            out.line_width = model.line_width;
            out.clip_position = vec4<f32>(model.position, 0.0, 1.0);
            return out;
        }
        "#
    }

    /// フラグメントシェーダーのソースコード（WGSL）
    fn fragment_shader_source() -> &'static str {
        r#"
        struct FragmentInput {
            @location(0) color: vec4<f32>,
            @location(1) line_width: f32,
        }

        @fragment
        fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
            // アンチエイリアシングのための簡単な処理
            var alpha = in.color.a;
            
            // 線の幅に応じたアルファ調整（将来の拡張用）
            if (in.line_width < 1.0) {
                alpha = alpha * in.line_width;
            }
            
            return vec4<f32>(in.color.rgb, alpha);
        }
        "#
    }
}

impl Drop for BasicDrawPipeline {
    fn drop(&mut self) {
        debug!("[BasicDrawPipeline] パイプラインを解放中");
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
    fn test_vertex2d_creation() {
        let vertex = Vertex2D::new(0.5, -0.3, [1.0, 0.0, 0.0, 1.0], 2.0);
        assert_eq!(vertex.position, [0.5, -0.3]);
        assert_eq!(vertex.color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(vertex.line_width, 2.0);
    }

    #[test]
    fn test_draw_stroke_creation() {
        let mut stroke = DrawStroke::new([0.0, 1.0, 0.0, 1.0], 3.0);
        assert_eq!(stroke.points.len(), 0);
        assert_eq!(stroke.color, [0.0, 1.0, 0.0, 1.0]);
        assert_eq!(stroke.base_width, 3.0);
        assert!(!stroke.is_closed);

        stroke.add_point(0.0, 0.0, 1.0);
        stroke.add_point(1.0, 1.0, 0.5);
        assert_eq!(stroke.points.len(), 2);
        
        let triangles = stroke.to_triangles();
        assert_eq!(triangles.len(), 6); // 1線分 = 2三角形 = 6頂点
    }

    #[test]
    fn test_coordinate_conversion() {
        let screen_size = (800, 600);
        
        // 中央点のテスト
        let center_screen = (400.0, 300.0);
        let center_norm = BasicDrawPipeline::screen_to_normalized(center_screen, screen_size);
        assert!((center_norm.0 - 0.0).abs() < 1e-6);
        assert!((center_norm.1 - 0.0).abs() < 1e-6);
        
        // 逆変換のテスト
        let back_to_screen = BasicDrawPipeline::normalized_to_screen(center_norm, screen_size);
        assert!((back_to_screen.0 - center_screen.0).abs() < 1e-3);
        assert!((back_to_screen.1 - center_screen.1).abs() < 1e-3);
        
        // 左上角のテスト
        let top_left_screen = (0.0, 0.0);
        let top_left_norm = BasicDrawPipeline::screen_to_normalized(top_left_screen, screen_size);
        assert!((top_left_norm.0 - (-1.0)).abs() < 1e-6);
        assert!((top_left_norm.1 - 1.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_pipeline_creation() {
        let (device, _queue) = create_test_device();
        let format = TextureFormat::Rgba8UnormSrgb;
        
        let pipeline = BasicDrawPipeline::new(&device, format);
        assert!(pipeline.is_ok());
        
        let pipeline = pipeline.unwrap();
        assert_eq!(pipeline.max_vertices, 10000);
    }

    #[test]
    fn test_vertex_layout() {
        let layout = Vertex2D::desc();
        assert_eq!(layout.array_stride, std::mem::size_of::<Vertex2D>() as u64);
        assert_eq!(layout.attributes.len(), 3);
        
        // Position attribute
        assert_eq!(layout.attributes[0].shader_location, 0);
        assert_eq!(layout.attributes[0].format, VertexFormat::Float32x2);
        
        // Color attribute  
        assert_eq!(layout.attributes[1].shader_location, 1);
        assert_eq!(layout.attributes[1].format, VertexFormat::Float32x4);
        
        // Line width attribute
        assert_eq!(layout.attributes[2].shader_location, 2);
        assert_eq!(layout.attributes[2].format, VertexFormat::Float32);
    }

    #[test]
    fn test_stroke_triangle_generation() {
        let mut stroke = DrawStroke::new([1.0, 0.0, 0.0, 1.0], 2.0);
        
        // 単一点の場合
        stroke.add_point(0.0, 0.0, 1.0);
        let triangles = stroke.to_triangles();
        assert_eq!(triangles.len(), 0); // 線分にならないため0
        
        // 2点の場合
        stroke.add_point(1.0, 0.0, 1.0);
        let triangles = stroke.to_triangles();
        assert_eq!(triangles.len(), 6); // 1線分 = 2三角形 = 6頂点
        
        // 3点の場合
        stroke.add_point(1.0, 1.0, 0.8);
        let triangles = stroke.to_triangles();
        assert_eq!(triangles.len(), 12); // 2線分 = 4三角形 = 12頂点
    }
}