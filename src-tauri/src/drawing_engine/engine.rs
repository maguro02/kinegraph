use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use super::{
    renderer::WgpuRenderer,
    canvas_state::{CanvasState, CanvasId},
    commands::{DrawCommand, BrushSettings},
    stroke::{StrokeState, draw_brush_stroke, draw_brush_point},
    compositor::Compositor,
};

pub struct DrawingEngine {
    renderer: Arc<WgpuRenderer>,
    canvases: Arc<Mutex<HashMap<CanvasId, CanvasState>>>,
    active_canvas: Arc<Mutex<Option<CanvasId>>>,
    brush_settings: Arc<Mutex<BrushSettings>>,
    stroke_states: Arc<Mutex<HashMap<CanvasId, StrokeState>>>,
}

impl DrawingEngine {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let renderer = Arc::new(WgpuRenderer::new().await?);
        
        Ok(Self {
            renderer,
            canvases: Arc::new(Mutex::new(HashMap::new())),
            active_canvas: Arc::new(Mutex::new(None)),
            brush_settings: Arc::new(Mutex::new(BrushSettings::default())),
            stroke_states: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    
    pub async fn create_canvas(&self, width: u32, height: u32) -> Result<CanvasId, String> {
        let canvas_id = CanvasId::new();
        let mut canvas = CanvasState::new(canvas_id.clone(), width, height);
        
        // wgpuテクスチャの作成
        let texture = self.renderer.create_texture(width, height);
        canvas.texture = Some(texture);
        
        let mut canvases = self.canvases.lock().await;
        canvases.insert(canvas_id.clone(), canvas);
        
        // ストローク状態を初期化
        let mut stroke_states = self.stroke_states.lock().await;
        stroke_states.insert(canvas_id.clone(), StrokeState::default());
        
        // アクティブキャンバスに設定
        let mut active = self.active_canvas.lock().await;
        *active = Some(canvas_id.clone());
        
        Ok(canvas_id)
    }
    
    pub async fn process_command(&self, command: DrawCommand) -> Result<(), String> {
        let active_canvas_id = {
            let active = self.active_canvas.lock().await;
            active.clone().ok_or("No active canvas")?
        };
        
        let mut canvases = self.canvases.lock().await;
        let canvas = canvases
            .get_mut(&active_canvas_id)
            .ok_or("Canvas not found")?;
        
        match command {
            DrawCommand::BeginStroke { x, y, pressure } => {
                self.begin_stroke(canvas, x, y, pressure).await?;
            }
            DrawCommand::ContinueStroke { x, y, pressure } => {
                self.continue_stroke(canvas, x, y, pressure).await?;
            }
            DrawCommand::EndStroke => {
                self.end_stroke(canvas).await?;
            }
            DrawCommand::Clear => {
                if let Some(layer) = canvas.get_active_layer() {
                    layer.clear();
                    canvas.dirty = true;
                }
            }
            DrawCommand::SetBrush(settings) => {
                let mut brush = self.brush_settings.lock().await;
                *brush = settings;
            }
            DrawCommand::SetActiveLayer(index) => {
                if index < canvas.layers.len() {
                    canvas.active_layer = index;
                } else {
                    return Err("Layer index out of bounds".to_string());
                }
            }
            DrawCommand::CreateLayer => {
                canvas.add_layer();
            }
            DrawCommand::DeleteLayer(index) => {
                canvas.delete_layer(index)?;
            }
        }
        
        // 変更があった場合、レンダリングを更新
        if canvas.dirty {
            self.update_canvas_texture(canvas).await?;
            canvas.dirty = false;
        }
        
        Ok(())
    }
    
    async fn begin_stroke(&self, canvas: &mut CanvasState, x: f32, y: f32, pressure: f32) -> Result<(), String> {
        let mut stroke_states = self.stroke_states.lock().await;
        if let Some(stroke_state) = stroke_states.get_mut(&canvas.id) {
            stroke_state.begin(x, y, pressure);
            
            // 最初の点を描画
            let brush = self.brush_settings.lock().await;
            if let Some(layer) = canvas.get_active_layer() {
                draw_brush_point(&mut layer.data, layer.width, layer.height, x, y, pressure, &brush);
                canvas.dirty = true;
            }
        }
        Ok(())
    }
    
    async fn continue_stroke(&self, canvas: &mut CanvasState, x: f32, y: f32, pressure: f32) -> Result<(), String> {
        let mut stroke_states = self.stroke_states.lock().await;
        if let Some(stroke_state) = stroke_states.get_mut(&canvas.id) {
            if stroke_state.is_drawing {
                if let Some((last_x, last_y)) = stroke_state.last_point {
                    let brush = self.brush_settings.lock().await;
                    if let Some(layer) = canvas.get_active_layer() {
                        // 前回の点からの線を補間して描画
                        let last_pressure = stroke_state.current_path.last()
                            .map(|(_, _, p)| *p)
                            .unwrap_or(pressure);
                        
                        draw_brush_stroke(
                            &mut layer.data,
                            layer.width,
                            layer.height,
                            last_x,
                            last_y,
                            x,
                            y,
                            last_pressure,
                            pressure,
                            &brush,
                        );
                        
                        canvas.dirty = true;
                    }
                }
                
                stroke_state.add_point(x, y, pressure);
            }
        }
        
        Ok(())
    }
    
    async fn end_stroke(&self, canvas: &mut CanvasState) -> Result<(), String> {
        let mut stroke_states = self.stroke_states.lock().await;
        if let Some(stroke_state) = stroke_states.get_mut(&canvas.id) {
            stroke_state.end();
        }
        Ok(())
    }
    
    async fn update_canvas_texture(&self, canvas: &mut CanvasState) -> Result<(), String> {
        if let Some(texture) = &canvas.texture {
            // 全レイヤーを合成
            let mut composite_buffer = vec![0u8; (canvas.width * canvas.height * 4) as usize];
            
            // レイヤーとブレンドモードのペアを作成（将来使用予定）
            let _layers_with_blend: Vec<_> = canvas.layers.iter()
                .map(|layer| (layer, layer.blend_mode.clone()))
                .collect();
            
            // 合成処理
            Compositor::composite_layers(
                &canvas.layers,
                &mut composite_buffer,
                canvas.width,
                canvas.height,
            )?;
            
            // 合成結果をテクスチャにアップロード
            self.renderer.upload_layer_data(
                texture,
                &composite_buffer,
                canvas.width,
                canvas.height,
            );
        }
        
        Ok(())
    }
    
    pub async fn get_canvas_data(&self, canvas_id: &CanvasId) -> Result<(Vec<u8>, u32, u32), String> {
        let canvases = self.canvases.lock().await;
        let canvas = canvases.get(canvas_id).ok_or("Canvas not found")?;
        
        if let Some(texture) = &canvas.texture {
            let data = self.renderer
                .render_to_buffer(texture, canvas.width, canvas.height)
                .await
                .map_err(|e| e.to_string())?;
            Ok((data, canvas.width, canvas.height))
        } else {
            Err("Canvas texture not initialized".to_string())
        }
    }
    
    pub async fn resize_canvas(&self, canvas_id: &CanvasId, width: u32, height: u32) -> Result<(), String> {
        let mut canvases = self.canvases.lock().await;
        let canvas = canvases.get_mut(canvas_id).ok_or("Canvas not found")?;
        
        canvas.resize(width, height);
        
        // 新しいテクスチャを作成
        let texture = self.renderer.create_texture(width, height);
        canvas.texture = Some(texture);
        
        Ok(())
    }
}