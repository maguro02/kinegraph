use wasm_bindgen::prelude::*;
use web_sys::OffscreenCanvas;
use wgpu::util::DeviceExt;

mod worker;
pub use worker::*;

mod types;
pub use types::*;

mod draw_engine;
pub use draw_engine::*;

mod shaders;
pub use shaders::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Setup panic hook for better error messages in console
pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

// Shader code for stroke rendering
const STROKE_SHADER: &str = r#"
struct Uniforms {
    canvas_size: vec2<f32>,
    stroke_color: vec4<f32>,
    stroke_width: f32,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
}

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
) -> VertexOutput {
    // Convert from pixel coordinates to NDC
    let ndc_x = (position.x / uniforms.canvas_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (position.y / uniforms.canvas_size.y) * 2.0;
    
    var output: VertexOutput;
    output.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    output.tex_coord = tex_coord;
    return output;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return uniforms.stroke_color;
}
"#;

// Vertex data for stroke points
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct StrokeVertex {
    position: [f32; 2],
    tex_coord: [f32; 2],
}

impl StrokeVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2
    ];
    
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<StrokeVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

// Uniform buffer data
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct StrokeUniforms {
    canvas_size: [f32; 2],
    stroke_color: [f32; 4],
    stroke_width: f32,
    _padding: f32,
}

// WebGPU Renderer structure
#[wasm_bindgen]
pub struct WebGPURenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
    stroke_pipeline: Option<wgpu::RenderPipeline>,
    uniform_bind_group_layout: Option<wgpu::BindGroupLayout>,
}

#[wasm_bindgen]
impl WebGPURenderer {
    // Initialize WebGPU with proper feature detection
    pub async fn new(canvas: Option<OffscreenCanvas>) -> Result<WebGPURenderer, JsValue> {
        set_panic_hook();
        console_log::init_with_level(log::Level::Debug).expect("Failed to init logger");
        
        log::info!("Initializing WebGPU renderer...");
        
        // Request adapter with WebGPU features
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            ..Default::default()
        });
        
        let surface = if let Some(_canvas) = canvas {
            // Note: OffscreenCanvas support in wgpu is limited
            // For now, we'll use offscreen rendering without a surface
            None
        } else {
            None
        };
        
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: surface.as_ref(),
            })
            .await
            .ok_or_else(|| JsValue::from_str("Failed to find suitable GPU adapter"))?;
        
        log::info!("Adapter found: {:?}", adapter.get_info());
        
        // Request device with all features we need
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Kinegraph WebGPU Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to create device: {:?}", e)))?;
        
        log::info!("Device created successfully");
        
        let config = if let Some(ref surface) = surface {
            let size = surface.get_capabilities(&adapter).formats[0];
            Some(wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: size,
                width: 800,  // Default size, will be updated
                height: 600, // Default size, will be updated
                present_mode: wgpu::PresentMode::AutoVsync,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            })
        } else {
            None
        };
        
        if let (Some(ref surface), Some(ref config)) = (&surface, &config) {
            surface.configure(&device, config);
        }
        
        Ok(WebGPURenderer {
            device,
            queue,
            surface,
            config,
            stroke_pipeline: None,
            uniform_bind_group_layout: None,
        })
    }
    
    // Render a frame
    pub fn render(&mut self) -> Result<(), JsValue> {
        if let Some(ref surface) = self.surface {
            let output = surface.get_current_texture()
                .map_err(|e| JsValue::from_str(&format!("Failed to get surface texture: {:?}", e)))?;
            
            let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
            
            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
            
            {
                let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }
            
            self.queue.submit(std::iter::once(encoder.finish()));
            output.present();
        }
        
        Ok(())
    }
    
    // Resize the rendering surface
    pub fn resize(&mut self, width: u32, height: u32) {
        if let Some(ref mut config) = self.config {
            config.width = width;
            config.height = height;
            
            if let Some(ref surface) = self.surface {
                surface.configure(&self.device, config);
            }
        }
    }
}

// SharedArrayBuffer support for high-performance data transfer
#[wasm_bindgen]
pub struct SharedBuffer {
    buffer: js_sys::SharedArrayBuffer,
}

#[wasm_bindgen]
impl SharedBuffer {
    #[wasm_bindgen(constructor)]
    pub fn new(size: u32) -> Result<SharedBuffer, JsValue> {
        let buffer = js_sys::SharedArrayBuffer::new(size);
        Ok(SharedBuffer { buffer })
    }
    
    #[wasm_bindgen(getter)]
    pub fn buffer(&self) -> js_sys::SharedArrayBuffer {
        self.buffer.clone()
    }
    
    pub fn write_pixels(&self, offset: u32, pixels: &[u8]) -> Result<(), JsValue> {
        let uint8_array = js_sys::Uint8Array::new(&self.buffer);
        uint8_array.set(&js_sys::Uint8Array::from(pixels), offset);
        Ok(())
    }
}

// Drawing context for offscreen rendering
#[wasm_bindgen]
pub struct DrawingContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    width: u32,
    height: u32,
    render_texture: wgpu::Texture,
    render_view: wgpu::TextureView,
    stroke_pipeline: wgpu::RenderPipeline,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    output_buffer: wgpu::Buffer,
    shared_buffer: Option<SharedBuffer>,
}

#[wasm_bindgen]
impl DrawingContext {
    pub async fn new(width: u32, height: u32) -> Result<DrawingContext, JsValue> {
        set_panic_hook();
        console_log::init_with_level(log::Level::Info).ok();
        
        log::info!("Creating DrawingContext {}x{}", width, height);
        
        // Validate dimensions
        if width == 0 || height == 0 {
            return Err(JsValue::from_str("Canvas dimensions must be greater than zero"));
        }
        
        // Initialize WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            ..Default::default()
        });
        
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .ok_or_else(|| JsValue::from_str("Failed to find suitable GPU adapter"))?;
        
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Drawing Context Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to create device: {:?}", e)))?;
        
        // Create render texture
        let render_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Render Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        
        let render_view = render_texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // Create shader module
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Stroke Shader"),
            source: wgpu::ShaderSource::Wgsl(STROKE_SHADER.into()),
        });
        
        // Create bind group layout for uniforms
        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        
        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Stroke Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create render pipeline
        let stroke_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Stroke Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[StrokeVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        
        // Create output buffer for reading pixels
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: (width * height * 4) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        
        Ok(DrawingContext {
            device,
            queue,
            width,
            height,
            render_texture,
            render_view,
            stroke_pipeline,
            uniform_bind_group_layout,
            output_buffer,
            shared_buffer: None,
        })
    }
    
    pub fn draw_stroke(&mut self, points: &[f32], color: &[f32], width: f32) -> Result<(), JsValue> {
        // Validate inputs early to avoid null pointer issues
        if points.is_empty() {
            return Err(JsValue::from_str("Points array is empty"));
        }
        
        if points.len() < 4 {
            return Err(JsValue::from_str("Need at least 2 points to draw a stroke"));
        }
        
        if points.len() % 2 != 0 {
            return Err(JsValue::from_str("Points array must have even number of values (x,y pairs)"));
        }
        
        // Validate all point values
        for (i, &val) in points.iter().enumerate() {
            if val.is_nan() || val.is_infinite() {
                return Err(JsValue::from_str(&format!("Invalid point value at index {}: {}", i, val)));
            }
        }
        
        log::info!("Drawing stroke with {} points, width: {}, color: {:?}", points.len() / 2, width, color);
        
        // Validate color array
        if color.len() < 4 {
            return Err(JsValue::from_str("Color array must have at least 4 values (RGBA)"));
        }
        
        // Validate stroke width
        if width <= 0.0 || width > 100.0 || width.is_nan() || width.is_infinite() {
            return Err(JsValue::from_str(&format!("Invalid stroke width: {}", width)));
        }
        
        // Convert points to vertices with thickness
        let mut vertices = Vec::new();
        let half_width = width * 0.5;
        
        for i in (0..points.len()).step_by(2) {
            let x = points[i];
            let y = points[i + 1];
            
            // Calculate perpendicular direction for thickness
            let (dx, dy) = if i + 3 < points.len() {
                let next_x = points[i + 2];
                let next_y = points[i + 3];
                let dx = next_x - x;
                let dy = next_y - y;
                let len = (dx * dx + dy * dy).sqrt();
                if len > 0.0 {
                    (-dy / len, dx / len)
                } else {
                    (0.0, 1.0)
                }
            } else if i >= 2 {
                let prev_x = points[i - 2];
                let prev_y = points[i - 1];
                let dx = x - prev_x;
                let dy = y - prev_y;
                let len = (dx * dx + dy * dy).sqrt();
                if len > 0.0 {
                    (-dy / len, dx / len)
                } else {
                    (0.0, 1.0)
                }
            } else {
                (0.0, 1.0)
            };
            
            // Add two vertices for the stroke width
            vertices.push(StrokeVertex {
                position: [x - dx * half_width, y - dy * half_width],
                tex_coord: [0.0, 0.0],
            });
            vertices.push(StrokeVertex {
                position: [x + dx * half_width, y + dy * half_width],
                tex_coord: [1.0, 0.0],
            });
        }
        
        if vertices.is_empty() {
            return Ok(());
        }
        
        // Create vertex buffer
        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Stroke Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        // Create uniform buffer
        let uniforms = StrokeUniforms {
            canvas_size: [self.width as f32, self.height as f32],
            stroke_color: [
                color.get(0).copied().unwrap_or(0.0),
                color.get(1).copied().unwrap_or(0.0),
                color.get(2).copied().unwrap_or(0.0),
                color.get(3).copied().unwrap_or(1.0),
            ],
            stroke_width: width,
            _padding: 0.0,
        };
        
        let uniform_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Stroke Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Stroke Bind Group"),
            layout: &self.uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });
        
        // Create command encoder and render
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Stroke Render Encoder"),
        });
        
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Stroke Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.render_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            
            render_pass.set_pipeline(&self.stroke_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.draw(0..vertices.len() as u32, 0..1);
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
        
        Ok(())
    }
    
    pub fn clear(&mut self, color: &[f32]) -> Result<(), JsValue> {
        log::debug!("Clearing with color: {:?}", color);
        
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Clear Encoder"),
        });
        
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.render_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: color.get(0).copied().unwrap_or(1.0) as f64,
                            g: color.get(1).copied().unwrap_or(1.0) as f64,
                            b: color.get(2).copied().unwrap_or(1.0) as f64,
                            a: color.get(3).copied().unwrap_or(1.0) as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
        
        Ok(())
    }
    
    pub fn get_pixels(&self) -> Result<Vec<u8>, JsValue> {
        log::debug!("Reading pixels from texture");
        
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Readback Encoder"),
        });
        
        // Copy texture to buffer
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &self.render_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &self.output_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(self.width * 4),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        
        self.queue.submit(std::iter::once(encoder.finish()));
        
        // Wait for GPU to finish
        let (tx, rx) = std::sync::mpsc::channel();
        let buffer_slice = self.output_buffer.slice(..);
        
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        
        self.device.poll(wgpu::Maintain::Wait);
        
        rx.recv()
            .map_err(|_| JsValue::from_str("Failed to receive buffer map result"))?
            .map_err(|_| JsValue::from_str("Failed to map buffer"))?;
        
        let data = buffer_slice.get_mapped_range();
        let pixels = data.to_vec();
        
        drop(data);
        self.output_buffer.unmap();
        
        Ok(pixels)
    }
    
    // Set a shared buffer for efficient pixel data transfer
    pub fn set_shared_buffer(&mut self, buffer: SharedBuffer) {
        self.shared_buffer = Some(buffer);
    }
    
    // Copy pixels to shared buffer if available
    pub fn copy_to_shared_buffer(&self) -> Result<(), JsValue> {
        if let Some(ref shared_buffer) = self.shared_buffer {
            let pixels = self.get_pixels()?;
            shared_buffer.write_pixels(0, &pixels)?;
        }
        Ok(())
    }
    
    // Resize the drawing context
    // New method: Direct Float32Array handling for better performance
    pub fn draw_stroke_from_typed_array(&mut self, points: js_sys::Float32Array, color: js_sys::Float32Array, width: f32) -> Result<(), JsValue> {
        // Convert typed arrays to vectors with validation
        let points_vec = points.to_vec();
        if points_vec.is_empty() {
            return Err(JsValue::from_str("Points array is empty"));
        }
        
        let color_vec = color.to_vec();
        if color_vec.len() < 4 {
            return Err(JsValue::from_str("Color array must have at least 4 values (RGBA)"));
        }
        
        self.draw_stroke(&points_vec, &color_vec, width)
    }
    
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), JsValue> {
        // Validate dimensions
        if width == 0 || height == 0 || width > 8192 || height > 8192 {
            return Err(JsValue::from_str("Invalid canvas dimensions"));
        }
        
        self.width = width;
        self.height = height;
        
        // Recreate render texture with new size
        self.render_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Render Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        
        self.render_view = self.render_texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // Recreate output buffer with new size
        self.output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: (width * height * 4) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        
        Ok(())
    }
}

// Module initialization
#[wasm_bindgen(start)]
pub fn main() {
    set_panic_hook();
    console_log::init_with_level(log::Level::Debug).expect("Failed to init logger");
    log::info!("Kinegraph WASM module initialized");
}

// Utility function to check WebGPU support
#[wasm_bindgen]
pub fn check_webgpu_support() -> bool {
    let window = web_sys::window().expect("no global `window` exists");
    let navigator = window.navigator();
    
    // Check if gpu property exists on navigator
    let gpu = js_sys::Reflect::get(&navigator, &JsValue::from_str("gpu"))
        .unwrap_or(JsValue::UNDEFINED);
    
    !gpu.is_undefined()
}

// Export types for TypeScript
#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"
export interface DrawingPoint {
    x: number;
    y: number;
    pressure: number;
}

export interface StrokeData {
    points: Float32Array;
    color: [number, number, number, number];
    width: number;
}

export interface DrawingOptions {
    width: number;
    height: number;
    useSharedBuffer?: boolean;
}

export interface RenderStats {
    drawCalls: number;
    vertices: number;
    frameTime: number;
}
"#;