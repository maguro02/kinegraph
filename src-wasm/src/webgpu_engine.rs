use wasm_bindgen::prelude::*;
use wgpu::util::DeviceExt;
use js_sys::{SharedArrayBuffer, Uint8Array};
use std::collections::HashMap;

use crate::types::{Stroke, ActiveStroke, DirtyRegion, Point, Color, BrushType};
use crate::shaders::STROKE_SHADER;

// Vertex data for stroke rendering
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

/// WebGPU-accelerated drawing engine with SharedArrayBuffer support
#[wasm_bindgen]
pub struct WebGPUDrawEngine {
    // WebGPU resources
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_texture: wgpu::Texture,
    render_view: wgpu::TextureView,
    stroke_pipeline: wgpu::RenderPipeline,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    output_buffer: wgpu::Buffer,
    
    // SharedArrayBuffer for pixel data
    shared_buffer: SharedArrayBuffer,
    pixel_data: Uint8Array,
    
    // Stroke management
    strokes: HashMap<u32, Stroke>,
    next_stroke_id: u32,
    active_stroke: Option<ActiveStroke>,
    
    // Canvas properties
    canvas_width: u32,
    canvas_height: u32,
    
    // Dirty regions for incremental updates
    dirty_regions: Vec<DirtyRegion>,
    
    // Caching for performance
    vertex_buffer_cache: HashMap<u32, wgpu::Buffer>,
}

#[wasm_bindgen]
impl WebGPUDrawEngine {
    /// Create a new WebGPU-accelerated drawing engine
    #[wasm_bindgen(constructor)]
    pub async fn new(canvas_width: u32, canvas_height: u32) -> Result<WebGPUDrawEngine, JsValue> {
        console_log::init_with_level(log::Level::Info).ok();
        
        log::info!("Initializing WebGPU Drawing Engine {}x{}", canvas_width, canvas_height);
        
        // Validate dimensions
        if canvas_width == 0 || canvas_height == 0 || canvas_width > 8192 || canvas_height > 8192 {
            return Err(JsValue::from_str("Invalid canvas dimensions"));
        }
        
        // Create SharedArrayBuffer
        let buffer_size = (canvas_width * canvas_height * 4) as usize;
        let shared_buffer = SharedArrayBuffer::new(buffer_size as u32);
        let pixel_data = Uint8Array::new(&shared_buffer);
        
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
                    label: Some("WebGPU Drawing Engine"),
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
                width: canvas_width,
                height: canvas_height,
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
            size: (canvas_width * canvas_height * 4) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        
        Ok(WebGPUDrawEngine {
            device,
            queue,
            render_texture,
            render_view,
            stroke_pipeline,
            uniform_bind_group_layout,
            output_buffer,
            shared_buffer,
            pixel_data,
            strokes: HashMap::new(),
            next_stroke_id: 1,
            active_stroke: None,
            canvas_width,
            canvas_height,
            dirty_regions: Vec::new(),
            vertex_buffer_cache: HashMap::new(),
        })
    }
    
    /// Get the SharedArrayBuffer
    #[wasm_bindgen(getter)]
    pub fn shared_buffer(&self) -> SharedArrayBuffer {
        self.shared_buffer.clone()
    }
    
    /// Begin a new stroke
    #[wasm_bindgen]
    pub fn begin_stroke(&mut self, x: f32, y: f32, pressure: f32, r: u8, g: u8, b: u8, a: u8, brush_type: u32, size: f32) -> Result<u32, JsValue> {
        let point = Point { x, y, pressure };
        let color = Color { 
            r: r as f32 / 255.0, 
            g: g as f32 / 255.0, 
            b: b as f32 / 255.0, 
            a: a as f32 / 255.0 
        };
        let brush_type = match brush_type {
            0 => BrushType::Pen,
            1 => BrushType::Eraser,
            _ => return Err(JsValue::from_str("Invalid brush type")),
        };
        
        let stroke_id = self.next_stroke_id;
        self.next_stroke_id += 1;
        
        self.active_stroke = Some(ActiveStroke {
            id: stroke_id,
            points: vec![point],
            color,
            brush_type,
            size,
        });
        
        // Mark region as dirty
        self.add_dirty_region(x - size, y - size, size * 2.0, size * 2.0);
        
        Ok(stroke_id)
    }
    
    /// Add a point to the active stroke
    #[wasm_bindgen]
    pub fn add_point(&mut self, x: f32, y: f32, pressure: f32) -> Result<(), JsValue> {
        let (should_render, size) = if let Some(ref mut active_stroke) = self.active_stroke {
            let point = Point { x, y, pressure };
            active_stroke.points.push(point);
            
            // Check if we should render incrementally
            (active_stroke.points.len() >= 2, active_stroke.size)
        } else {
            (false, 0.0)
        };
        
        if should_render {
            // Mark region as dirty
            self.add_dirty_region(x - size, y - size, size * 2.0, size * 2.0);
            
            // Render incrementally
            self.render_active_stroke_incremental()?;
        }
        
        Ok(())
    }
    
    /// End the current stroke
    #[wasm_bindgen]
    pub fn end_stroke(&mut self) -> Result<(), JsValue> {
        if let Some(active_stroke) = self.active_stroke.take() {
            let stroke = Stroke {
                id: active_stroke.id,
                points: active_stroke.points,
                color: active_stroke.color,
                brush_type: active_stroke.brush_type.clone(),
                size: active_stroke.size,
            };
            
            // Create and cache vertex buffer for this stroke
            if let Ok(vertex_buffer) = self.create_stroke_vertex_buffer(&stroke) {
                self.vertex_buffer_cache.insert(stroke.id, vertex_buffer);
            }
            
            self.strokes.insert(stroke.id, stroke);
        }
        Ok(())
    }
    
    /// Clear the canvas
    #[wasm_bindgen]
    pub fn clear(&mut self) -> Result<(), JsValue> {
        self.strokes.clear();
        self.active_stroke = None;
        self.vertex_buffer_cache.clear();
        self.dirty_regions.clear();
        
        // Clear with white background
        self.clear_render_texture()?;
        self.copy_to_shared_buffer()?;
        
        Ok(())
    }
    
    /// Render the current state to the SharedArrayBuffer
    #[wasm_bindgen]
    pub fn render(&mut self) -> Result<(), JsValue> {
        // If we have dirty regions, render only those areas
        if !self.dirty_regions.is_empty() {
            self.render_dirty_regions()?;
            self.dirty_regions.clear();
        } else {
            // Full render
            self.render_full()?;
        }
        
        // Copy rendered texture to SharedArrayBuffer
        self.copy_to_shared_buffer()?;
        
        Ok(())
    }
    
    /// Resize the canvas
    #[wasm_bindgen]
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), JsValue> {
        if width == 0 || height == 0 || width > 8192 || height > 8192 {
            return Err(JsValue::from_str("Invalid canvas dimensions"));
        }
        
        self.canvas_width = width;
        self.canvas_height = height;
        
        // Recreate SharedArrayBuffer
        let buffer_size = (width * height * 4) as usize;
        self.shared_buffer = SharedArrayBuffer::new(buffer_size as u32);
        self.pixel_data = Uint8Array::new(&self.shared_buffer);
        
        // Recreate render texture
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
        
        // Recreate output buffer
        self.output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: (width * height * 4) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        
        // Mark entire canvas as dirty
        self.dirty_regions.clear();
        self.dirty_regions.push(DirtyRegion {
            x: 0,
            y: 0,
            width,
            height,
        });
        
        Ok(())
    }
    
    // Private helper methods
    
    fn add_dirty_region(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let region = DirtyRegion {
            x: x.max(0.0) as u32,
            y: y.max(0.0) as u32,
            width: width.ceil() as u32,
            height: height.ceil() as u32,
        };
        
        // Merge with existing regions if overlapping
        // For now, just add it
        self.dirty_regions.push(region);
    }
    
    fn create_stroke_vertex_buffer(&self, stroke: &Stroke) -> Result<wgpu::Buffer, JsValue> {
        let mut vertices = Vec::new();
        let half_width = stroke.size * 0.5;
        
        // Convert stroke points to vertices
        for i in 0..stroke.points.len() {
            let point = &stroke.points[i];
            
            // Calculate perpendicular direction
            let (dx, dy) = if i + 1 < stroke.points.len() {
                let next = &stroke.points[i + 1];
                let dx = next.x - point.x;
                let dy = next.y - point.y;
                let len = (dx * dx + dy * dy).sqrt();
                if len > 0.0 {
                    (-dy / len, dx / len)
                } else {
                    (0.0, 1.0)
                }
            } else if i > 0 {
                let prev = &stroke.points[i - 1];
                let dx = point.x - prev.x;
                let dy = point.y - prev.y;
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
                position: [point.x - dx * half_width, point.y - dy * half_width],
                tex_coord: [0.0, 0.0],
            });
            vertices.push(StrokeVertex {
                position: [point.x + dx * half_width, point.y + dy * half_width],
                tex_coord: [1.0, 0.0],
            });
        }
        
        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Stroke Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        Ok(vertex_buffer)
    }
    
    fn render_active_stroke_incremental(&mut self) -> Result<(), JsValue> {
        if let Some(ref active_stroke) = self.active_stroke {
            if active_stroke.points.len() < 2 {
                return Ok(());
            }
            
            // Render only the last segment
            let last_idx = active_stroke.points.len() - 1;
            let p0 = &active_stroke.points[last_idx - 1];
            let p1 = &active_stroke.points[last_idx];
            
            // Create vertices for this segment
            let half_width = active_stroke.size * 0.5;
            let dx = p1.x - p0.x;
            let dy = p1.y - p0.y;
            let len = (dx * dx + dy * dy).sqrt();
            let (nx, ny) = if len > 0.0 {
                (-dy / len, dx / len)
            } else {
                (0.0, 1.0)
            };
            
            let vertices = vec![
                StrokeVertex {
                    position: [p0.x - nx * half_width, p0.y - ny * half_width],
                    tex_coord: [0.0, 0.0],
                },
                StrokeVertex {
                    position: [p0.x + nx * half_width, p0.y + ny * half_width],
                    tex_coord: [1.0, 0.0],
                },
                StrokeVertex {
                    position: [p1.x - nx * half_width, p1.y - ny * half_width],
                    tex_coord: [0.0, 1.0],
                },
                StrokeVertex {
                    position: [p1.x + nx * half_width, p1.y + ny * half_width],
                    tex_coord: [1.0, 1.0],
                },
            ];
            
            let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Segment Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            
            // Render this segment
            self.render_stroke_segment(&vertex_buffer, &active_stroke.color, active_stroke.size)?;
            
            // Immediately copy affected region to SharedArrayBuffer
            let region = DirtyRegion {
                x: (p0.x.min(p1.x) - active_stroke.size).max(0.0) as u32,
                y: (p0.y.min(p1.y) - active_stroke.size).max(0.0) as u32,
                width: ((p0.x - p1.x).abs() + active_stroke.size * 2.0).ceil() as u32,
                height: ((p0.y - p1.y).abs() + active_stroke.size * 2.0).ceil() as u32,
            };
            
            self.copy_region_to_shared_buffer(&region)?;
        }
        
        Ok(())
    }
    
    fn render_stroke_segment(&self, vertex_buffer: &wgpu::Buffer, color: &Color, width: f32) -> Result<(), JsValue> {
        let uniforms = StrokeUniforms {
            canvas_size: [self.canvas_width as f32, self.canvas_height as f32],
            stroke_color: [color.r, color.g, color.b, color.a],
            stroke_width: width,
            _padding: 0.0,
        };
        
        let uniform_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Stroke Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
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
            render_pass.draw(0..4, 0..1);
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
        
        Ok(())
    }
    
    fn clear_render_texture(&self) -> Result<(), JsValue> {
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
                            r: 1.0,
                            g: 1.0,
                            b: 1.0,
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
        
        Ok(())
    }
    
    fn render_full(&mut self) -> Result<(), JsValue> {
        self.clear_render_texture()?;
        
        // Render all strokes
        for (stroke_id, stroke) in &self.strokes {
            if let Some(vertex_buffer) = self.vertex_buffer_cache.get(stroke_id) {
                self.render_stroke_with_buffer(vertex_buffer, &stroke.color, stroke.size)?;
            }
        }
        
        // Render active stroke
        if let Some(ref active_stroke) = self.active_stroke {
            if let Ok(vertex_buffer) = self.create_stroke_vertex_buffer(&Stroke {
                id: active_stroke.id,
                points: active_stroke.points.clone(),
                color: active_stroke.color.clone(),
                brush_type: active_stroke.brush_type.clone(),
                size: active_stroke.size,
            }) {
                self.render_stroke_with_buffer(&vertex_buffer, &active_stroke.color, active_stroke.size)?;
            }
        }
        
        Ok(())
    }
    
    fn render_stroke_with_buffer(&self, vertex_buffer: &wgpu::Buffer, color: &Color, width: f32) -> Result<(), JsValue> {
        let uniforms = StrokeUniforms {
            canvas_size: [self.canvas_width as f32, self.canvas_height as f32],
            stroke_color: [color.r, color.g, color.b, color.a],
            stroke_width: width,
            _padding: 0.0,
        };
        
        let uniform_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Stroke Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
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
        
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Stroke Render Encoder"),
        });
        
        // Get vertex count from buffer size
        let vertex_count = vertex_buffer.size() / std::mem::size_of::<StrokeVertex>() as u64;
        
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
            render_pass.draw(0..vertex_count as u32, 0..1);
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
        
        Ok(())
    }
    
    fn render_dirty_regions(&mut self) -> Result<(), JsValue> {
        // For now, just do a full render
        // TODO: Implement partial rendering
        self.render_full()
    }
    
    fn copy_to_shared_buffer(&self) -> Result<(), JsValue> {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Readback Encoder"),
        });
        
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
                    bytes_per_row: Some(self.canvas_width * 4),
                    rows_per_image: Some(self.canvas_height),
                },
            },
            wgpu::Extent3d {
                width: self.canvas_width,
                height: self.canvas_height,
                depth_or_array_layers: 1,
            },
        );
        
        self.queue.submit(std::iter::once(encoder.finish()));
        
        // Map buffer and copy to SharedArrayBuffer
        let buffer_slice = self.output_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        
        self.device.poll(wgpu::Maintain::Wait);
        
        rx.recv()
            .map_err(|_| JsValue::from_str("Failed to receive buffer map result"))?
            .map_err(|_| JsValue::from_str("Failed to map buffer"))?;
        
        let data = buffer_slice.get_mapped_range();
        
        // Copy to SharedArrayBuffer
        for (i, &byte) in data.iter().enumerate() {
            self.pixel_data.set_index(i as u32, byte);
        }
        
        drop(data);
        self.output_buffer.unmap();
        
        Ok(())
    }
    
    fn copy_region_to_shared_buffer(&self, _region: &DirtyRegion) -> Result<(), JsValue> {
        // For now, copy the entire buffer
        // TODO: Implement partial copying
        self.copy_to_shared_buffer()
    }
}