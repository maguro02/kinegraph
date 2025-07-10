use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use wasm_bindgen::prelude::*;
use js_sys::{Uint8Array, SharedArrayBuffer};
use web_sys::console;
use crate::types::{Stroke, ActiveStroke, DirtyRegion, Point, Color, BrushType, BrushStamp, StampCache};
use crate::stroke::{create_brush_stamp, get_or_create_stamp};

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
    
    /// スタンプキャッシュ
    stamp_cache: StampCache,
    
    /// ストロークレンダリングキャッシュ
    stroke_render_cache: HashMap<u32, StrokeRenderCache>,
}

/// ストロークのレンダリングキャッシュ
#[derive(Debug, Clone)]
struct StrokeRenderCache {
    /// 最後にレンダリングされたポイント数
    last_rendered_point: usize,
    /// 最後にレンダリングされたポイントの座標
    last_point: Option<Point>,
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
        
        // バッファを白で初期化
        for i in 0..buffer_size {
            pixel_data.set_index(i as u32, 255);
        }
        
        Ok(DrawEngine {
            strokes: HashMap::new(),
            next_stroke_id: 1,
            active_stroke: None,
            canvas_width,
            canvas_height,
            dirty_regions: Vec::new(),
            shared_buffer,
            pixel_data,
            stamp_cache: Arc::new(Mutex::new(HashMap::new())),
            stroke_render_cache: HashMap::new(),
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
            
            // バッファを白で初期化（resizeの場合、以前の内容は失われる）
            for i in 0..buffer_size {
                self.pixel_data.set_index(i as u32, 255);
            }
            
            // ストロークレンダリングキャッシュをクリア（サイズが変わったため）
            self.stroke_render_cache.clear();
            
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
        // アクティブストロークの情報をコピー
        let stroke_info = if let Some(ref mut active_stroke) = self.active_stroke {
            let last_point = active_stroke.points.last().cloned();
            active_stroke.points.push(point);
            Some((last_point, active_stroke.color, active_stroke.size))
        } else {
            None
        };
        
        // インクリメンタルレンダリング
        if let Some((Some(last_point), color, size)) = stroke_info {
            self.render_incremental_stroke(&last_point, &point, &color, size);
        }
        
        // ダーティリージョンを更新
        if let Some(ref active_stroke) = self.active_stroke {
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
            let stroke_id = active_stroke.id;
            let stroke = Stroke {
                id: active_stroke.id,
                points: active_stroke.points,
                color: active_stroke.color,
                brush_type: active_stroke.brush_type,
                size: active_stroke.size,
            };
            
            // レンダリングキャッシュを初期化
            self.stroke_render_cache.insert(stroke_id, StrokeRenderCache {
                last_rendered_point: stroke.points.len(),
                last_point: stroke.points.last().cloned(),
            });
            
            self.strokes.insert(stroke.id, stroke);
        }
    }
    
    /// ストロークを削除 (WASM API)
    #[wasm_bindgen]
    pub fn remove_stroke(&mut self, stroke_id: u32) -> bool {
        let removed = self.strokes.remove(&stroke_id).is_some();
        if removed {
            // レンダリングキャッシュも削除
            self.stroke_render_cache.remove(&stroke_id);
        }
        removed
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
        self.stroke_render_cache.clear();
        
        // バッファをクリア
        self.clear_buffer();
        
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
    
    /// 現在のダーティリージョンを取得してクリア (WASM API)
    #[wasm_bindgen]
    pub fn get_and_clear_dirty_regions(&mut self) -> Result<js_sys::Array, JsValue> {
        let regions = js_sys::Array::new();
        
        // ダーティリージョンをマージして最適化
        let merged_region = self.merge_dirty_regions();
        
        if let Some(region) = merged_region {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &"x".into(), &region.x.into())?;
            js_sys::Reflect::set(&obj, &"y".into(), &region.y.into())?;
            js_sys::Reflect::set(&obj, &"width".into(), &region.width.into())?;
            js_sys::Reflect::set(&obj, &"height".into(), &region.height.into())?;
            regions.push(&obj);
        }
        
        // ダーティリージョンをクリア
        self.dirty_regions.clear();
        
        Ok(regions)
    }
    
    /// ダーティリージョンをマージして最適化
    fn merge_dirty_regions(&self) -> Option<DirtyRegion> {
        if self.dirty_regions.is_empty() {
            return None;
        }
        
        let mut merged = self.dirty_regions[0];
        for region in &self.dirty_regions[1..] {
            merged = merged.merge(region);
        }
        
        // キャンバスの境界内に収める
        merged.x = merged.x.min(self.canvas_width.saturating_sub(1));
        merged.y = merged.y.min(self.canvas_height.saturating_sub(1));
        merged.width = merged.width.min(self.canvas_width - merged.x);
        merged.height = merged.height.min(self.canvas_height - merged.y);
        
        Some(merged)
    }
    
    /// 描画を実行してSharedArrayBufferに書き込み
    #[wasm_bindgen]
    pub fn render(&mut self) -> Result<(), JsValue> {
        // SharedArrayBufferに描画結果を書き込む
        self.render_to_shared_buffer()?;
        Ok(())
    }
    
    /// 部分的に描画を実行（ダーティリージョンのみ）
    #[wasm_bindgen]
    pub fn render_dirty_regions(&mut self) -> Result<(), JsValue> {
        // ダーティリージョンがない場合は何もしない
        if self.dirty_regions.is_empty() {
            return Ok(());
        }
        
        // ダーティリージョンをマージ
        if let Some(region) = self.merge_dirty_regions() {
            self.render_region(&region)?;
        }
        
        Ok(())
    }
    
    /// SharedArrayBufferに描画
    fn render_to_shared_buffer(&mut self) -> Result<(), JsValue> {
        #[cfg(debug_assertions)]
        {
            console::log_1(&format!("render_to_shared_buffer: canvas size = {}x{}", self.canvas_width, self.canvas_height).into());
            console::log_1(&format!("render_to_shared_buffer: buffer size = {}", self.shared_buffer.byte_length()).into());
        }
        
        // 注意: clear_buffer()を削除しました。
        // バッファのクリアはclear()メソッドが明示的に呼ばれた時のみ実行されます。
        // これにより、既存のストロークが消える問題を解決します。
        
        // 各ストロークを描画
        let stroke_ids: Vec<u32> = self.strokes.keys().cloned().collect();
        #[cfg(debug_assertions)]
        console::log_1(&format!("render_to_shared_buffer: rendering {} strokes", stroke_ids.len()).into());
        
        for stroke_id in stroke_ids {
            if let Some(stroke) = self.strokes.get(&stroke_id) {
                #[cfg(debug_assertions)]
                console::log_1(&format!("render_to_shared_buffer: rendering stroke {} with {} points", stroke_id, stroke.points.len()).into());
                let stroke_clone = stroke.clone();
                self.render_stroke(&stroke_clone)?;
            }
        }
        
        // アクティブなストロークも描画
        if let Some(active_stroke) = self.active_stroke.clone() {
            #[cfg(debug_assertions)]
            console::log_1(&format!("render_to_shared_buffer: rendering active stroke with {} points", active_stroke.points.len()).into());
            self.render_active_stroke(&active_stroke)?;
        }
        
        // バッファの最初の数ピクセルをチェック
        let mut non_white_pixels = 0;
        for i in 0..100.min((self.canvas_width * self.canvas_height) as usize) {
            let index = (i * 4) as u32;
            let r = self.pixel_data.get_index(index);
            let g = self.pixel_data.get_index(index + 1);
            let b = self.pixel_data.get_index(index + 2);
            if r != 255 || g != 255 || b != 255 {
                non_white_pixels += 1;
            }
        }
        #[cfg(debug_assertions)]
        console::log_1(&format!("render_to_shared_buffer: {} non-white pixels in first 100 pixels", non_white_pixels).into());
        
        #[cfg(debug_assertions)]
        console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Rendered {} strokes to SharedArrayBuffer", self.strokes.len())));
        Ok(())
    }
    
    /// バッファをクリア
    fn clear_buffer(&mut self) {
        let buffer_length = (self.canvas_width * self.canvas_height * 4) as usize;
        // より効率的な方法: fill()を使用
        for i in 0..buffer_length {
            self.pixel_data.set_index(i as u32, 255);
        }
        
        // レンダリングキャッシュもクリア
        self.stroke_render_cache.clear();
    }
    
    /// 特定の領域のみを描画
    fn render_region(&mut self, region: &DirtyRegion) -> Result<(), JsValue> {
        // 領域内のバッファをクリア
        self.clear_region(region);
        
        // 領域と交差するストロークのみを描画
        let stroke_ids: Vec<u32> = self.strokes.keys().cloned().collect();
        for stroke_id in stroke_ids {
            if let Some(stroke) = self.strokes.get(&stroke_id) {
                // ストロークのバウンディングボックスと領域が交差するかチェック
                if let Some((min_point, max_point)) = stroke.bounding_box() {
                    let stroke_region = DirtyRegion {
                        x: min_point.x as u32,
                        y: min_point.y as u32,
                        width: (max_point.x - min_point.x) as u32,
                        height: (max_point.y - min_point.y) as u32,
                    };
                    
                    if region.intersects(&stroke_region) {
                        let stroke_clone = stroke.clone();
                        self.render_stroke_in_region(&stroke_clone, region)?;
                    }
                }
            }
        }
        
        // アクティブなストロークも描画
        if let Some(active_stroke) = self.active_stroke.clone() {
            // 簡易的にアクティブストロークは常に描画
            self.render_active_stroke_in_region(&active_stroke, region)?;
        }
        
        Ok(())
    }
    
    /// 特定の領域をクリア
    fn clear_region(&mut self, region: &DirtyRegion) {
        let x_start = region.x;
        let x_end = (region.x + region.width).min(self.canvas_width);
        let y_start = region.y;
        let y_end = (region.y + region.height).min(self.canvas_height);
        
        // 最適化: 行ごとに処理
        for y in y_start..y_end {
            let row_start = ((y * self.canvas_width + x_start) * 4) as u32;
            let row_end = ((y * self.canvas_width + x_end) * 4) as u32;
            
            for index in (row_start..row_end).step_by(4) {
                self.pixel_data.set_index(index, 255);     // R
                self.pixel_data.set_index(index + 1, 255); // G
                self.pixel_data.set_index(index + 2, 255); // B
                self.pixel_data.set_index(index + 3, 255); // A
            }
        }
    }
    
    /// ストロークを描画
    fn render_stroke(&mut self, stroke: &Stroke) -> Result<(), JsValue> {
        // スタンプベース描画を使用
        if stroke.points.len() < 2 {
            return Ok(());
        }
        
        // レンダリングキャッシュをチェック
        let start_index = if let Some(cache) = self.stroke_render_cache.get(&stroke.id) {
            // 既にレンダリングされた部分はスキップ
            cache.last_rendered_point.saturating_sub(1)
        } else {
            0
        };
        
        // 新しい部分のみをレンダリング
        for i in start_index.max(1)..stroke.points.len() {
            self.draw_line_with_stamps(
                &stroke.points[i - 1],
                &stroke.points[i],
                &stroke.color,
                stroke.size,
            )?;
        }
        
        // 最後のポイントにもスタンプを配置（新しい場合のみ）
        if start_index < stroke.points.len() {
            if let Some(last_point) = stroke.points.last() {
                self.draw_stamp(last_point.x, last_point.y, stroke.size, &stroke.color)?;
            }
        }
        
        // レンダリングキャッシュを更新
        self.stroke_render_cache.insert(stroke.id, StrokeRenderCache {
            last_rendered_point: stroke.points.len(),
            last_point: stroke.points.last().cloned(),
        });
        
        Ok(())
    }
    
    /// アクティブストロークを描画
    fn render_active_stroke(&mut self, stroke: &ActiveStroke) -> Result<(), JsValue> {
        if stroke.points.len() < 2 {
            return Ok(());
        }
        
        // スタンプベース描画を使用
        for i in 1..stroke.points.len() {
            self.draw_line_with_stamps(
                &stroke.points[i - 1],
                &stroke.points[i],
                &stroke.color,
                stroke.size,
            )?;
        }
        
        // 最後のポイントにもスタンプを配置
        if let Some(last_point) = stroke.points.last() {
            self.draw_stamp(last_point.x, last_point.y, stroke.size, &stroke.color)?;
        }
        
        Ok(())
    }
    
    /// ストロークを特定の領域内で描画
    fn render_stroke_in_region(&mut self, stroke: &Stroke, region: &DirtyRegion) -> Result<(), JsValue> {
        if stroke.points.len() < 2 {
            return Ok(());
        }
        
        // スタンプベース描画を使用（領域内のみ）
        for i in 1..stroke.points.len() {
            self.draw_line_with_stamps_in_region(
                &stroke.points[i - 1],
                &stroke.points[i],
                &stroke.color,
                stroke.size,
                region,
            )?;
        }
        
        // 最後のポイントにもスタンプを配置（領域内の場合）
        if let Some(last_point) = stroke.points.last() {
            let half_size = stroke.size / 2.0;
            let bounds = DirtyRegion {
                x: (last_point.x - half_size).max(0.0) as u32,
                y: (last_point.y - half_size).max(0.0) as u32,
                width: stroke.size.ceil() as u32,
                height: stroke.size.ceil() as u32,
            };
            
            if region.intersects(&bounds) {
                self.draw_stamp(last_point.x, last_point.y, stroke.size, &stroke.color)?;
            }
        }
        
        Ok(())
    }
    
    /// アクティブストロークを特定の領域内で描画
    fn render_active_stroke_in_region(&mut self, stroke: &ActiveStroke, region: &DirtyRegion) -> Result<(), JsValue> {
        if stroke.points.len() < 2 {
            return Ok(());
        }
        
        // スタンプベース描画を使用（領域内のみ）
        for i in 1..stroke.points.len() {
            self.draw_line_with_stamps_in_region(
                &stroke.points[i - 1],
                &stroke.points[i],
                &stroke.color,
                stroke.size,
                region,
            )?;
        }
        
        // 最後のポイントにもスタンプを配置（領域内の場合）
        if let Some(last_point) = stroke.points.last() {
            let half_size = stroke.size / 2.0;
            let bounds = DirtyRegion {
                x: (last_point.x - half_size).max(0.0) as u32,
                y: (last_point.y - half_size).max(0.0) as u32,
                width: stroke.size.ceil() as u32,
                height: stroke.size.ceil() as u32,
            };
            
            if region.intersects(&bounds) {
                self.draw_stamp(last_point.x, last_point.y, stroke.size, &stroke.color)?;
            }
        }
        
        Ok(())
    }
    
    /// スタンプベースの線描画（領域内のみ）
    fn draw_line_with_stamps_in_region(&mut self, p0: &Point, p1: &Point, color: &Color, size: f32, region: &DirtyRegion) -> Result<(), JsValue> {
        let distance = p0.distance_to(p1);
        if distance < 0.1 {
            return Ok(());
        }
        
        // 速度に基づく間隔の調整
        let velocity = distance;
        let velocity_factor = (velocity / 10.0).clamp(0.5, 2.0);
        let base_spacing = size * 0.25; // ブラシサイズの1/4
        let spacing = base_spacing * velocity_factor;
        
        // スタンプ数を計算（最低2つ）
        let num_stamps = ((distance / spacing).ceil() as usize).max(2);
        
        // 各スタンプを配置（領域内のみ）
        for i in 0..num_stamps {
            let t = i as f32 / (num_stamps - 1).max(1) as f32;
            let x = p0.x + (p1.x - p0.x) * t;
            let y = p0.y + (p1.y - p0.y) * t;
            let pressure = p0.pressure + (p1.pressure - p0.pressure) * t;
            
            // 筆圧を考慮したサイズ
            let stamp_size = size * pressure;
            let half_size = stamp_size / 2.0;
            
            // スタンプの境界ボックスを計算
            let stamp_bounds = DirtyRegion {
                x: (x - half_size).max(0.0) as u32,
                y: (y - half_size).max(0.0) as u32,
                width: stamp_size.ceil() as u32,
                height: stamp_size.ceil() as u32,
            };
            
            // 領域と交差する場合のみ描画
            if region.intersects(&stamp_bounds) {
                self.draw_stamp(x, y, stamp_size, color)?;
            }
        }
        
        Ok(())
    }
    
    /// 線を描画（ブレゼンハムアルゴリズム）
    fn draw_line(&mut self, p0: &Point, p1: &Point, color: &Color, size: f32) -> Result<(), JsValue> {
        #[cfg(debug_assertions)]
        console::log_1(&format!("draw_line: from ({}, {}) to ({}, {}), size={}", p0.x, p0.y, p1.x, p1.y, size).into());
        
        let dx = (p1.x - p0.x).abs();
        let dy = (p1.y - p0.y).abs();
        let sx = if p0.x < p1.x { 1.0 } else { -1.0 };
        let sy = if p0.y < p1.y { 1.0 } else { -1.0 };
        let mut err = dx - dy;
        
        let mut x = p0.x;
        let mut y = p0.y;
        let mut points_drawn = 0;
        
        loop {
            // 現在の点に円を描画
            self.draw_circle(x, y, size / 2.0, color)?;
            points_drawn += 1;
            
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
        
        #[cfg(debug_assertions)]
        console::log_1(&format!("draw_line: {} points drawn", points_drawn).into());
        
        Ok(())
    }
    
    /// 円を描画
    fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, color: &Color) -> Result<(), JsValue> {
        let x_start = (cx - radius).max(0.0) as u32;
        let x_end = ((cx + radius).ceil() as u32).min(self.canvas_width);
        let y_start = (cy - radius).max(0.0) as u32;
        let y_end = ((cy + radius).ceil() as u32).min(self.canvas_height);
        
        #[cfg(debug_assertions)]
        {
            console::log_1(&format!("draw_circle: center=({}, {}), radius={}, color=({}, {}, {}, {})", cx, cy, radius, color.r, color.g, color.b, color.a).into());
            console::log_1(&format!("draw_circle: bounds=({}, {}) to ({}, {})", x_start, y_start, x_end, y_end).into());
        }
        
        let mut pixels_drawn = 0;
        for y in y_start..y_end {
            for x in x_start..x_end {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let distance = (dx * dx + dy * dy).sqrt();
                
                if distance <= radius {
                    let index = ((y * self.canvas_width + x) * 4) as u32;
                    // アルファブレンディング
                    let alpha = color.a;
                    
                    // 効率化: 一度に4つの値を読み込み/書き込み
                    let current_r = self.pixel_data.get_index(index);
                    let current_g = self.pixel_data.get_index(index + 1);
                    let current_b = self.pixel_data.get_index(index + 2);
                    
                    let new_r = (color.r * 255.0 * alpha + current_r as f32 * (1.0 - alpha)) as u8;
                    let new_g = (color.g * 255.0 * alpha + current_g as f32 * (1.0 - alpha)) as u8;
                    let new_b = (color.b * 255.0 * alpha + current_b as f32 * (1.0 - alpha)) as u8;
                    
                    self.pixel_data.set_index(index, new_r);
                    self.pixel_data.set_index(index + 1, new_g);
                    self.pixel_data.set_index(index + 2, new_b);
                    self.pixel_data.set_index(index + 3, 255);
                    
                    pixels_drawn += 1;
                }
            }
        }
        
        #[cfg(debug_assertions)]
        console::log_1(&format!("draw_circle: {} pixels drawn", pixels_drawn).into());
        
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
    
    /// 線を特定の領域内で描画
    fn draw_line_in_region(&mut self, p0: &Point, p1: &Point, color: &Color, size: f32, region: &DirtyRegion) -> Result<(), JsValue> {
        let dx = (p1.x - p0.x).abs();
        let dy = (p1.y - p0.y).abs();
        let sx = if p0.x < p1.x { 1.0 } else { -1.0 };
        let sy = if p0.y < p1.y { 1.0 } else { -1.0 };
        let mut err = dx - dy;
        
        let mut x = p0.x;
        let mut y = p0.y;
        
        loop {
            // 現在の点に円を描画（領域内のみ）
            self.draw_circle_in_region(x, y, size / 2.0, color, region)?;
            
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
    
    /// スタンプベースの線描画
    fn draw_line_with_stamps(&mut self, p0: &Point, p1: &Point, color: &Color, size: f32) -> Result<(), JsValue> {
        let distance = p0.distance_to(p1);
        if distance < 0.1 {
            return Ok(());
        }
        
        // 速度に基づく間隔の調整
        let velocity = distance;
        let velocity_factor = (velocity / 10.0).clamp(0.5, 2.0);
        let base_spacing = size * 0.25; // ブラシサイズの1/4
        let spacing = base_spacing * velocity_factor;
        
        // スタンプ数を計算（最低2つ）
        let num_stamps = ((distance / spacing).ceil() as usize).max(2);
        
        // 各スタンプを配置
        for i in 0..num_stamps {
            let t = i as f32 / (num_stamps - 1).max(1) as f32;
            let x = p0.x + (p1.x - p0.x) * t;
            let y = p0.y + (p1.y - p0.y) * t;
            let pressure = p0.pressure + (p1.pressure - p0.pressure) * t;
            
            // 筆圧を考慮したサイズ
            let stamp_size = size * pressure;
            self.draw_stamp(x, y, stamp_size, color)?;
        }
        
        Ok(())
    }
    
    /// スタンプを描画
    fn draw_stamp(&mut self, cx: f32, cy: f32, size: f32, color: &Color) -> Result<(), JsValue> {
        let stamp_size = size.ceil() as u32;
        let stamp = get_or_create_stamp(&self.stamp_cache, stamp_size);
        
        let half_size = stamp.size as f32 / 2.0;
        let x_start = (cx - half_size).max(0.0) as u32;
        let x_end = ((cx + half_size).ceil() as u32).min(self.canvas_width);
        let y_start = (cy - half_size).max(0.0) as u32;
        let y_end = ((cy + half_size).ceil() as u32).min(self.canvas_height);
        
        for y in y_start..y_end {
            for x in x_start..x_end {
                let stamp_x = (x as f32 - cx + half_size) as u32;
                let stamp_y = (y as f32 - cy + half_size) as u32;
                
                if stamp_x < stamp.size && stamp_y < stamp.size {
                    let stamp_alpha = stamp.alpha_map[(stamp_y * stamp.size + stamp_x) as usize];
                    if stamp_alpha > 0.0 {
                        let index = ((y * self.canvas_width + x) * 4) as u32;
                        
                        // アルファブレンディング
                        let alpha = color.a * stamp_alpha;
                        let current_r = self.pixel_data.get_index(index);
                        let current_g = self.pixel_data.get_index(index + 1);
                        let current_b = self.pixel_data.get_index(index + 2);
                        
                        let new_r = (color.r * 255.0 * alpha + current_r as f32 * (1.0 - alpha)) as u8;
                        let new_g = (color.g * 255.0 * alpha + current_g as f32 * (1.0 - alpha)) as u8;
                        let new_b = (color.b * 255.0 * alpha + current_b as f32 * (1.0 - alpha)) as u8;
                        
                        self.pixel_data.set_index(index, new_r);
                        self.pixel_data.set_index(index + 1, new_g);
                        self.pixel_data.set_index(index + 2, new_b);
                        self.pixel_data.set_index(index + 3, 255);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// インクリメンタルストローク描画（最後のポイントから新しいポイントへの線のみ）
    fn render_incremental_stroke(&mut self, last_point: &Point, new_point: &Point, color: &Color, size: f32) {
        // スタンプベースの線描画を使用
        if let Err(_e) = self.draw_line_with_stamps(last_point, new_point, color, size) {
            #[cfg(debug_assertions)]
            console::log_1(&format!("render_incremental_stroke error: {:?}", _e).into());
        }
    }
    
    /// 円を特定の領域内で描画
    fn draw_circle_in_region(&mut self, cx: f32, cy: f32, radius: f32, color: &Color, region: &DirtyRegion) -> Result<(), JsValue> {
        let x_start = ((cx - radius).max(0.0) as u32).max(region.x);
        let x_end = ((cx + radius).ceil() as u32).min(self.canvas_width).min(region.x + region.width);
        let y_start = ((cy - radius).max(0.0) as u32).max(region.y);
        let y_end = ((cy + radius).ceil() as u32).min(self.canvas_height).min(region.y + region.height);
        
        for y in y_start..y_end {
            for x in x_start..x_end {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let distance = (dx * dx + dy * dy).sqrt();
                
                if distance <= radius {
                    let index = ((y * self.canvas_width + x) * 4) as u32;
                    // アルファブレンディング
                    let alpha = color.a;
                    let current_r = self.pixel_data.get_index(index);
                    let current_g = self.pixel_data.get_index(index + 1);
                    let current_b = self.pixel_data.get_index(index + 2);
                    
                    let new_r = (color.r * 255.0 * alpha + current_r as f32 * (1.0 - alpha)) as u8;
                    let new_g = (color.g * 255.0 * alpha + current_g as f32 * (1.0 - alpha)) as u8;
                    let new_b = (color.b * 255.0 * alpha + current_b as f32 * (1.0 - alpha)) as u8;
                    
                    self.pixel_data.set_index(index, new_r);
                    self.pixel_data.set_index(index + 1, new_g);
                    self.pixel_data.set_index(index + 2, new_b);
                    self.pixel_data.set_index(index + 3, 255); // Alpha
                }
            }
        }
        
        Ok(())
    }
}
