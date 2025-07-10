use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use super::{
    renderer::WgpuRenderer,
    canvas_state::{CanvasState, CanvasId},
    commands::{DrawEngineCommand, BrushSettings},
    stroke::{StrokeState, draw_brush_point, create_brush_stamp, draw_line_incremental, DirtyRegion},
    compositor::Compositor,
    types::StampCache,
};

pub struct DrawingEngine {
    renderer: Arc<WgpuRenderer>,
    canvases: Arc<Mutex<HashMap<CanvasId, CanvasState>>>,
    active_canvas: Arc<Mutex<Option<CanvasId>>>,
    brush_settings: Arc<Mutex<BrushSettings>>,
    stroke_states: Arc<Mutex<HashMap<CanvasId, StrokeState>>>,
    stamp_cache: StampCache,
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
            stamp_cache: Arc::new(std::sync::Mutex::new(HashMap::new())),
        })
    }
    
    /// バッチコマンドを処理（ワーカープール無しの簡易実装）
    pub async fn process_command_batch(&self, canvas_id: &CanvasId, commands: Vec<DrawEngineCommand>) -> Result<(), String> {
        // アクティブキャンバスを設定
        self.set_active_canvas(canvas_id.clone()).await?;
        
        let mut canvases = self.canvases.lock().await;
        let mut canvas = canvases
            .get_mut(canvas_id)
            .ok_or("Canvas not found")?;
        
        // レンダリング更新を一時停止
        let initial_dirty_state = canvas.dirty;
        
        // コマンドを処理（レンダリング更新なし）
        for command in commands {
            match command {
                DrawEngineCommand::BeginStroke { x, y, pressure } => {
                    drop(canvases); // 一時的にロックを解放
                    self.begin_stroke_internal(canvas_id, x, y, pressure).await?;
                    canvases = self.canvases.lock().await;
                    canvas = canvases.get_mut(canvas_id).ok_or("Canvas not found")?;
                }
                DrawEngineCommand::ContinueStroke { x, y, pressure } => {
                    drop(canvases); // 一時的にロックを解放
                    self.continue_stroke_internal(canvas_id, x, y, pressure).await?;
                    canvases = self.canvases.lock().await;
                    canvas = canvases.get_mut(canvas_id).ok_or("Canvas not found")?;
                }
                DrawEngineCommand::EndStroke => {
                    drop(canvases); // 一時的にロックを解放
                    self.end_stroke_internal(canvas_id).await?;
                    canvases = self.canvases.lock().await;
                    canvas = canvases.get_mut(canvas_id).ok_or("Canvas not found")?;
                }
                DrawEngineCommand::Clear => {
                    if let Some(layer) = canvas.get_active_layer() {
                        layer.clear();
                        canvas.dirty = true;
                    }
                }
                DrawEngineCommand::SetBrush(settings) => {
                    let mut brush = self.brush_settings.lock().await;
                    *brush = settings;
                }
                DrawEngineCommand::SetActiveLayer(index) => {
                    if index < canvas.layers.len() {
                        canvas.active_layer = index;
                    } else {
                        return Err("Layer index out of bounds".to_string());
                    }
                }
                DrawEngineCommand::CreateLayer => {
                    canvas.add_layer();
                }
                DrawEngineCommand::DeleteLayer(index) => {
                    canvas.delete_layer(index)?;
                }
            }
        }
        
        // バッチ処理後に一度だけレンダリング更新
        if canvas.dirty || initial_dirty_state {
            self.update_canvas_texture(canvas).await?;
            canvas.dirty = false;
        }
        
        Ok(())
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
    
    pub async fn process_command(&self, command: DrawEngineCommand) -> Result<(), String> {
        let active_canvas_id = {
            let active = self.active_canvas.lock().await;
            active.clone().ok_or("No active canvas")?
        };
        
        let mut canvases = self.canvases.lock().await;
        let canvas = canvases
            .get_mut(&active_canvas_id)
            .ok_or("Canvas not found")?;
        
        // ストローク関連のコマンドか判定
        let _is_stroke_command = matches!(
            command,
            DrawEngineCommand::BeginStroke { .. } | 
            DrawEngineCommand::ContinueStroke { .. } |
            DrawEngineCommand::EndStroke
        );
        
        match command {
            DrawEngineCommand::BeginStroke { x, y, pressure } => {
                log::debug!("[RS] Processing BeginStroke at ({}, {})", x, y);
                self.begin_stroke(canvas, x, y, pressure).await?;
                // ストローク開始時もレンダリング更新
                if canvas.dirty {
                    log::debug!("[RS] Canvas is dirty after BeginStroke, updating texture");
                    self.update_canvas_texture(canvas).await?;
                    canvas.dirty = false;
                }
            }
            DrawEngineCommand::ContinueStroke { x, y, pressure } => {
                log::debug!("[RS] Processing ContinueStroke at ({}, {})", x, y);
                self.continue_stroke(canvas, x, y, pressure).await?;
                // ストローク中も定期的にレンダリング更新を行う
                if canvas.dirty {
                    log::debug!("[RS] Canvas is dirty, updating texture during stroke");
                    self.update_canvas_texture(canvas).await?;
                    canvas.dirty = false;
                }
            }
            DrawEngineCommand::EndStroke => {
                log::debug!("[RS] Processing EndStroke");
                self.end_stroke(canvas).await?;
                // ストローク終了時のレンダリング更新
                if canvas.dirty {
                    log::debug!("[RS] Canvas is dirty after EndStroke, updating texture");
                    self.update_canvas_texture(canvas).await?;
                    canvas.dirty = false;
                }
            }
            DrawEngineCommand::Clear => {
                if let Some(layer) = canvas.get_active_layer() {
                    layer.clear();
                    canvas.dirty = true;
                }
                // クリア時は即座にレンダリング更新
                if canvas.dirty {
                    self.update_canvas_texture(canvas).await?;
                    canvas.dirty = false;
                }
            }
            DrawEngineCommand::SetBrush(settings) => {
                let mut brush = self.brush_settings.lock().await;
                *brush = settings;
            }
            DrawEngineCommand::SetActiveLayer(index) => {
                if index < canvas.layers.len() {
                    canvas.active_layer = index;
                } else {
                    return Err("Layer index out of bounds".to_string());
                }
            }
            DrawEngineCommand::CreateLayer => {
                canvas.add_layer();
            }
            DrawEngineCommand::DeleteLayer(index) => {
                canvas.delete_layer(index)?;
                // レイヤー削除時は即座にレンダリング更新
                if canvas.dirty {
                    self.update_canvas_texture(canvas).await?;
                    canvas.dirty = false;
                }
            }
        }
        
        // ストローク中（ContinueStroke）はレンダリング更新をスキップ
        // EndStroke、Clear、DeleteLayerの場合は各処理内で更新済み
        
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
                // まずポイントを追加
                stroke_state.add_point(x, y, pressure);
                
                // インクリメンタルレンダリングを使用
                let brush = self.brush_settings.lock().await;
                if let Some(layer) = canvas.get_active_layer() {
                    if let Some(_dirty_region) = draw_line_incremental(
                        &mut layer.data,
                        layer.width,
                        layer.height,
                        stroke_state,
                        &brush,
                    ) {
                        canvas.dirty = true;
                    }
                }
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
    
    /// インクリメンタルレンダリング（差分更新）
    pub async fn render_incremental(&self, canvas_id: &CanvasId) -> Result<Option<DirtyRegion>, String> {
        let mut canvases = self.canvases.lock().await;
        let canvas = canvases.get_mut(canvas_id).ok_or("Canvas not found")?;
        
        let mut stroke_states = self.stroke_states.lock().await;
        if let Some(stroke_state) = stroke_states.get_mut(canvas_id) {
            if stroke_state.is_drawing && stroke_state.has_unrendered_points() {
                let brush = self.brush_settings.lock().await;
                if let Some(layer) = canvas.get_active_layer() {
                    // インクリメンタルレンダリング
                    if let Some(dirty_region) = draw_line_incremental(
                        &mut layer.data,
                        layer.width,
                        layer.height,
                        stroke_state,
                        &brush,
                    ) {
                        canvas.dirty = true;
                        
                        // 部分的なテクスチャ更新（将来的に実装）
                        // 現在は全体更新
                        drop(stroke_states);
                        self.update_canvas_texture(canvas).await?;
                        canvas.dirty = false;
                        
                        return Ok(Some(dirty_region));
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// 現在のストロークのダーティリージョンを取得
    pub async fn get_stroke_dirty_region(&self, canvas_id: &CanvasId) -> Option<DirtyRegion> {
        let stroke_states = self.stroke_states.lock().await;
        stroke_states.get(canvas_id)
            .and_then(|state| state.dirty_region.clone())
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
    
    pub fn get_renderer(&self) -> Arc<WgpuRenderer> {
        self.renderer.clone()
    }
    
    pub async fn set_active_canvas(&self, canvas_id: CanvasId) -> Result<(), String> {
        let canvases = self.canvases.lock().await;
        if !canvases.contains_key(&canvas_id) {
            return Err("Canvas not found".to_string());
        }
        
        let mut active = self.active_canvas.lock().await;
        *active = Some(canvas_id);
        Ok(())
    }
    
    pub async fn process_command_with_canvas(&self, canvas_id: &CanvasId, command: DrawEngineCommand) -> Result<(), String> {
        // アクティブキャンバスを一時的に設定
        self.set_active_canvas(canvas_id.clone()).await?;
        // コマンドを処理
        self.process_command(command).await
    }
    
    /// スタンプキャッシュのクリア
    pub fn clear_stamp_cache(&self) {
        let mut cache = self.stamp_cache.lock().unwrap();
        cache.clear();
    }
    
    /// 特定サイズのスタンプを事前生成
    pub fn preload_stamp(&self, size: u32) {
        let mut cache = self.stamp_cache.lock().unwrap();
        if !cache.contains_key(&size) {
            let stamp = create_brush_stamp(size);
            cache.insert(size, stamp);
        }
    }
    
    // 内部用メソッド（バッチ処理用）
    async fn begin_stroke_internal(&self, canvas_id: &CanvasId, x: f32, y: f32, pressure: f32) -> Result<(), String> {
        let mut canvases = self.canvases.lock().await;
        let canvas = canvases.get_mut(canvas_id).ok_or("Canvas not found")?;
        self.begin_stroke(canvas, x, y, pressure).await
    }
    
    async fn continue_stroke_internal(&self, canvas_id: &CanvasId, x: f32, y: f32, pressure: f32) -> Result<(), String> {
        let mut canvases = self.canvases.lock().await;
        let canvas = canvases.get_mut(canvas_id).ok_or("Canvas not found")?;
        self.continue_stroke(canvas, x, y, pressure).await
    }
    
    /// ストロークレンダリングキャッシュのクリア
    pub async fn clear_stroke_cache(&self, canvas_id: &CanvasId) -> Result<(), String> {
        let mut stroke_states = self.stroke_states.lock().await;
        if let Some(stroke_state) = stroke_states.get_mut(canvas_id) {
            stroke_state.last_rendered_index = 0;
            stroke_state.dirty_region = None;
        }
        Ok(())
    }
    
    /// アクティブストロークの最適化されたレンダリング
    pub async fn render_active_stroke(&self, canvas_id: &CanvasId) -> Result<(), String> {
        // インクリメンタルレンダリングを実行
        if let Some(_dirty_region) = self.render_incremental(canvas_id).await? {
            // レンダリングが実行された場合、成功を返す
            Ok(())
        } else {
            // レンダリングが不要だった場合
            Ok(())
        }
    }
    
    async fn end_stroke_internal(&self, canvas_id: &CanvasId) -> Result<(), String> {
        let mut canvases = self.canvases.lock().await;
        let canvas = canvases.get_mut(canvas_id).ok_or("Canvas not found")?;
        self.end_stroke(canvas).await
    }
}