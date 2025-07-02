use std::collections::HashMap;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use wasm_bindgen::prelude::*;
use js_sys::{Uint8Array, SharedArrayBuffer};
use web_sys::console;
use crate::types::{Stroke, ActiveStroke, DirtyRegion, Point, Color, BrushType};

/// WebGPU/wgpuベースの描画エンジン
#[wasm_bindgen]
pub struct DrawEngine {
    /// SharedArrayBuffer (ピクセルデータ共有用)
    shared_buffer: SharedArrayBuffer,
    /// SharedArrayBufferのUint8Arrayビュー
    pixel_data: Uint8Array,
    
    /// ストローク管理用HashMap（stroke_id -> Stroke）
    strokes: HashMap<u32, Stroke>,
    /// 次のストロークID
    next_stroke_id: u32,
    /// 現在アクティブなストローク
    active_stroke: Option<ActiveStroke>,
    
    /// キャンバスサイズ
    canvas_width: u32,
    canvas_height: u32,
    
    /// ダーティリージョン（再描画が必要な領域）
    dirty_regions: Vec<DirtyRegion>,
}

#[wasm_bindgen]
impl DrawEngine {
    /// 新しいDrawEngineインスタンスを作成 (WASM API)
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_width: u32, canvas_height: u32) -> Result<DrawEngine, JsValue> {
        // SharedArrayBufferの作成
        let buffer_size = (canvas_width * canvas_height * 4) as usize;
        let shared_buffer = SharedArrayBuffer::new(buffer_size as u32);
        let pixel_data = Uint8Array::new(&shared_buffer);
        
        // TODO: WebGPUの初期化は後で実装
        // 現在はダミーのDeviceとQueueを使用
        
        Ok(DrawEngine {
            strokes: HashMap::new(),
            next_stroke_id: 1,
            active_stroke: None,
            canvas_width,
            canvas_height,
            dirty_regions: Vec::new(),
            shared_buffer,
            pixel_data,
        })
    }
    
    /// WebGPUデバイスを初期化 (内部メソッド)
    async fn init_webgpu(
        canvas_width: u32,
        canvas_height: u32,
        surface: Surface<'static>,
    ) -> Result<(Device, Queue, Surface<'static>, SurfaceConfiguration), Box<dyn std::error::Error>> {
        // WebGPUインスタンスを作成
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        // アダプターを取得
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("Failed to find suitable adapter")?;
        
        // デバイスとキューを作成
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("DrawEngine Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await?;
        
        // サーフェース設定
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: canvas_width,
            height: canvas_height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        
        surface.configure(&device, &config);
        
        Ok((device, queue, surface, config))
    }
    
    /// キャンバスサイズを更新
    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width > 0 && new_height > 0 {
            self.canvas_width = new_width;
            self.canvas_height = new_height;
            
            // SharedArrayBufferを再作成
            let buffer_size = (new_width * new_height * 4) as usize;
            self.shared_buffer = SharedArrayBuffer::new(buffer_size as u32);
            self.pixel_data = Uint8Array::new(&self.shared_buffer);
            
            // 全体を再描画対象に
            self.dirty_regions.push(DirtyRegion {
                x: 0,
                y: 0,
                width: new_width,
                height: new_height,
            });
        }
    }
    
    /// 新しいストロークを開始 (WASM API)
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
        Ok(self.begin_stroke_internal(point, color, brush_type, size))
    }
    
    /// 新しいストロークを開始 (内部実装)
    fn begin_stroke_internal(&mut self, point: Point, color: Color, brush_type: BrushType, size: f32) -> u32 {
        let stroke_id = self.next_stroke_id;
        self.next_stroke_id += 1;
        
        self.active_stroke = Some(ActiveStroke {
            id: stroke_id,
            points: vec![point],
            color,
            brush_type,
            size,
        });
        
        stroke_id
    }
    
    /// アクティブなストロークにポイントを追加 (WASM API)
    #[wasm_bindgen]
    pub fn add_point(&mut self, x: f32, y: f32, pressure: f32) -> Result<(), JsValue> {
        let point = Point { x, y, pressure };
        self.add_point_to_stroke_internal(point);
        Ok(())
    }
    
    /// アクティブなストロークにポイントを追加 (内部実装)
    fn add_point_to_stroke_internal(&mut self, point: Point) {
        if let Some(ref mut active_stroke) = self.active_stroke {
            active_stroke.points.push(point);
            
            // ダーティリージョンを更新（簡易版）
            let region_size = (active_stroke.size * 2.0).ceil() as u32;
            self.dirty_regions.push(DirtyRegion {
                x: (point.x - active_stroke.size).max(0.0) as u32,
                y: (point.y - active_stroke.size).max(0.0) as u32,
                width: region_size,
                height: region_size,
            });
        }
    }
    
    /// ストロークを終了し、保存 (WASM API)
    #[wasm_bindgen]
    pub fn end_stroke(&mut self) -> Result<(), JsValue> {
        self.end_stroke_internal();
        Ok(())
    }
    
    /// ストロークを終了し、保存 (内部実装)
    fn end_stroke_internal(&mut self) {
        if let Some(active_stroke) = self.active_stroke.take() {
            let stroke = Stroke {
                id: active_stroke.id,
                points: active_stroke.points,
                color: active_stroke.color,
                brush_type: active_stroke.brush_type,
                size: active_stroke.size,
            };
            self.strokes.insert(stroke.id, stroke);
        }
    }
    
    /// ストロークを削除 (WASM API)
    #[wasm_bindgen]
    pub fn remove_stroke(&mut self, stroke_id: u32) -> bool {
        self.strokes.remove(&stroke_id).is_some()
    }
    
    /// 全ストロークをクリア (WASM API)
    #[wasm_bindgen]
    pub fn clear(&mut self) -> Result<(), JsValue> {
        self.clear_internal();
        Ok(())
    }
    
    /// 全ストロークをクリア (内部実装)
    fn clear_internal(&mut self) {
        self.strokes.clear();
        self.active_stroke = None;
        
        // 全体を再描画対象に
        self.dirty_regions.push(DirtyRegion {
            x: 0,
            y: 0,
            width: self.canvas_width,
            height: self.canvas_height,
        });
    }
    
    /// アンドゥ操作 (WASM API)
    #[wasm_bindgen]
    pub fn undo(&mut self) -> Result<bool, JsValue> {
        // 最後のストロークを削除
        if let Some((&last_id, _)) = self.strokes.iter().last() {
            self.strokes.remove(&last_id);
            self.mark_canvas_dirty();
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// リドゥ操作 (WASM API)
    #[wasm_bindgen]
    pub fn redo(&mut self) -> Result<bool, JsValue> {
        // TODO: リドゥスタックの実装が必要
        Ok(false)
    }
    
    /// SharedArrayBufferを取得 (WASM API)
    #[wasm_bindgen]
    pub fn get_shared_buffer(&self) -> SharedArrayBuffer {
        self.shared_buffer.clone()
    }
    
    /// 描画を実行してSharedArrayBufferに書き込み
    #[wasm_bindgen]
    pub fn render(&mut self) -> Result<(), JsValue> {
        // SharedArrayBufferに描画結果を書き込む
        self.render_to_shared_buffer()?;
        Ok(())
    }
    
    /// SharedArrayBufferに描画
    fn render_to_shared_buffer(&mut self) -> Result<(), JsValue> {
        // キャンバスをクリア
        self.clear_buffer();
        
        // 各ストロークを描画
        let stroke_ids: Vec<u32> = self.strokes.keys().cloned().collect();
        for stroke_id in stroke_ids {
            if let Some(stroke) = self.strokes.get(&stroke_id) {
                let stroke_clone = stroke.clone();
                self.render_stroke(&stroke_clone)?;
            }
        }
        
        // アクティブなストロークも描画
        if let Some(active_stroke) = self.active_stroke.clone() {
            self.render_active_stroke(&active_stroke)?;
        }
        
        console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Rendered {} strokes to SharedArrayBuffer", self.strokes.len())));
        Ok(())
    }
    
    /// バッファをクリア
    fn clear_buffer(&mut self) {
        let buffer_length = (self.canvas_width * self.canvas_height * 4) as usize;
        for i in (0..buffer_length).step_by(4) {
            // RGBA形式で白い背景
            self.pixel_data.set_index(i as u32, 255);     // R
            self.pixel_data.set_index(i as u32 + 1, 255); // G
            self.pixel_data.set_index(i as u32 + 2, 255); // B
            self.pixel_data.set_index(i as u32 + 3, 255); // A
        }
    }
    
    /// ストロークを描画
    fn render_stroke(&mut self, stroke: &Stroke) -> Result<(), JsValue> {
        // シンプルな線描画アルゴリズム
        for i in 1..stroke.points.len() {
            self.draw_line(
                &stroke.points[i - 1],
                &stroke.points[i],
                &stroke.color,
                stroke.size,
            )?;
        }
        Ok(())
    }
    
    /// アクティブストロークを描画
    fn render_active_stroke(&mut self, stroke: &ActiveStroke) -> Result<(), JsValue> {
        for i in 1..stroke.points.len() {
            self.draw_line(
                &stroke.points[i - 1],
                &stroke.points[i],
                &stroke.color,
                stroke.size,
            )?;
        }
        Ok(())
    }
    
    /// 線を描画（ブレゼンハムアルゴリズム）
    fn draw_line(&mut self, p0: &Point, p1: &Point, color: &Color, size: f32) -> Result<(), JsValue> {
        let dx = (p1.x - p0.x).abs();
        let dy = (p1.y - p0.y).abs();
        let sx = if p0.x < p1.x { 1.0 } else { -1.0 };
        let sy = if p0.y < p1.y { 1.0 } else { -1.0 };
        let mut err = dx - dy;
        
        let mut x = p0.x;
        let mut y = p0.y;
        
        loop {
            // 現在の点に円を描画
            self.draw_circle(x, y, size / 2.0, color)?;
            
            if (x - p1.x).abs() < 0.5 && (y - p1.y).abs() < 0.5 {
                break;
            }
            
            let e2 = 2.0 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
        
        Ok(())
    }
    
    /// 円を描画
    fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, color: &Color) -> Result<(), JsValue> {
        let x_start = (cx - radius).max(0.0) as u32;
        let x_end = ((cx + radius).ceil() as u32).min(self.canvas_width);
        let y_start = (cy - radius).max(0.0) as u32;
        let y_end = ((cy + radius).ceil() as u32).min(self.canvas_height);
        
        for y in y_start..y_end {
            for x in x_start..x_end {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let distance = (dx * dx + dy * dy).sqrt();
                
                if distance <= radius {
                    let index = ((y * self.canvas_width + x) * 4) as u32;
                    // アルファブレンディング
                    let alpha = color.a as f32 / 255.0;
                    let current_r = self.pixel_data.get_index(index);
                    let current_g = self.pixel_data.get_index(index + 1);
                    let current_b = self.pixel_data.get_index(index + 2);
                    
                    let new_r = (color.r as f32 * alpha + current_r as f32 * (1.0 - alpha)) as u8;
                    let new_g = (color.g as f32 * alpha + current_g as f32 * (1.0 - alpha)) as u8;
                    let new_b = (color.b as f32 * alpha + current_b as f32 * (1.0 - alpha)) as u8;
                    
                    self.pixel_data.set_index(index, new_r);
                    self.pixel_data.set_index(index + 1, new_g);
                    self.pixel_data.set_index(index + 2, new_b);
                    self.pixel_data.set_index(index + 3, 255);
                }
            }
        }
        
        Ok(())
    }
    
    /// キャンバス全体をダーティにマーク
    fn mark_canvas_dirty(&mut self) {
        self.dirty_regions.push(DirtyRegion {
            x: 0,
            y: 0,
            width: self.canvas_width,
            height: self.canvas_height,
        });
    }
}
