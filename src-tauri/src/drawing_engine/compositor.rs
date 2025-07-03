use super::layer::Layer;
use super::commands::BlendMode;

/// 複数のレイヤーを合成して最終的な画像を生成
pub struct Compositor;

impl Compositor {
    /// すべてのレイヤーを合成
    pub fn composite_layers(
        layers: &[Layer],
        output: &mut [u8],
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        if output.len() != (width * height * 4) as usize {
            return Err("Output buffer size mismatch".to_string());
        }
        
        // 出力バッファをクリア（透明な黒）
        output.fill(0);
        
        // 各レイヤーを順番に合成
        for layer in layers {
            if !layer.visible || layer.opacity <= 0.0 {
                continue;
            }
            
            Self::composite_layer(layer, output, width, height)?;
        }
        
        Ok(())
    }
    
    /// 単一レイヤーを出力バッファに合成
    fn composite_layer(
        layer: &Layer,
        output: &mut [u8],
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        if layer.width != width || layer.height != height {
            return Err("Layer size mismatch".to_string());
        }
        
        let opacity = layer.opacity;
        
        for y in 0..height {
            for x in 0..width {
                let offset = ((y * width + x) * 4) as usize;
                
                let src_r = layer.data[offset] as f32 / 255.0;
                let src_g = layer.data[offset + 1] as f32 / 255.0;
                let src_b = layer.data[offset + 2] as f32 / 255.0;
                let src_a = (layer.data[offset + 3] as f32 / 255.0) * opacity;
                
                if src_a <= 0.0 {
                    continue;
                }
                
                let dst_r = output[offset] as f32 / 255.0;
                let dst_g = output[offset + 1] as f32 / 255.0;
                let dst_b = output[offset + 2] as f32 / 255.0;
                let dst_a = output[offset + 3] as f32 / 255.0;
                
                // 通常のアルファブレンド（将来的にブレンドモードを追加）
                let out_a = src_a + dst_a * (1.0 - src_a);
                
                if out_a > 0.0 {
                    let out_r = (src_r * src_a + dst_r * dst_a * (1.0 - src_a)) / out_a;
                    let out_g = (src_g * src_a + dst_g * dst_a * (1.0 - src_a)) / out_a;
                    let out_b = (src_b * src_a + dst_b * dst_a * (1.0 - src_a)) / out_a;
                    
                    output[offset] = (out_r * 255.0) as u8;
                    output[offset + 1] = (out_g * 255.0) as u8;
                    output[offset + 2] = (out_b * 255.0) as u8;
                    output[offset + 3] = (out_a * 255.0) as u8;
                }
            }
        }
        
        Ok(())
    }
    
    /// ブレンドモードを考慮したレイヤー合成
    pub fn composite_layers_with_blend_modes(
        layers: &[(Layer, BlendMode)],
        output: &mut [u8],
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        if output.len() != (width * height * 4) as usize {
            return Err("Output buffer size mismatch".to_string());
        }
        
        // 出力バッファをクリア
        output.fill(0);
        
        for (layer, blend_mode) in layers {
            if !layer.visible || layer.opacity <= 0.0 {
                continue;
            }
            
            Self::composite_layer_with_blend_mode(layer, blend_mode, output, width, height)?;
        }
        
        Ok(())
    }
    
    /// ブレンドモードを適用してレイヤーを合成
    fn composite_layer_with_blend_mode(
        layer: &Layer,
        blend_mode: &BlendMode,
        output: &mut [u8],
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        if layer.width != width || layer.height != height {
            return Err("Layer size mismatch".to_string());
        }
        
        let opacity = layer.opacity;
        
        for y in 0..height {
            for x in 0..width {
                let offset = ((y * width + x) * 4) as usize;
                
                let src_r = layer.data[offset] as f32 / 255.0;
                let src_g = layer.data[offset + 1] as f32 / 255.0;
                let src_b = layer.data[offset + 2] as f32 / 255.0;
                let src_a = (layer.data[offset + 3] as f32 / 255.0) * opacity;
                
                if src_a <= 0.0 {
                    continue;
                }
                
                let dst_r = output[offset] as f32 / 255.0;
                let dst_g = output[offset + 1] as f32 / 255.0;
                let dst_b = output[offset + 2] as f32 / 255.0;
                let dst_a = output[offset + 3] as f32 / 255.0;
                
                let (blend_r, blend_g, blend_b) = match blend_mode {
                    BlendMode::Normal => (src_r, src_g, src_b),
                    BlendMode::Multiply => (
                        src_r * dst_r,
                        src_g * dst_g,
                        src_b * dst_b,
                    ),
                    BlendMode::Screen => (
                        1.0 - (1.0 - src_r) * (1.0 - dst_r),
                        1.0 - (1.0 - src_g) * (1.0 - dst_g),
                        1.0 - (1.0 - src_b) * (1.0 - dst_b),
                    ),
                    BlendMode::Overlay => (
                        if dst_r < 0.5 { 2.0 * src_r * dst_r } else { 1.0 - 2.0 * (1.0 - src_r) * (1.0 - dst_r) },
                        if dst_g < 0.5 { 2.0 * src_g * dst_g } else { 1.0 - 2.0 * (1.0 - src_g) * (1.0 - dst_g) },
                        if dst_b < 0.5 { 2.0 * src_b * dst_b } else { 1.0 - 2.0 * (1.0 - src_b) * (1.0 - dst_b) },
                    ),
                };
                
                // アルファブレンド
                let out_a = src_a + dst_a * (1.0 - src_a);
                
                if out_a > 0.0 {
                    let out_r = (blend_r * src_a + dst_r * dst_a * (1.0 - src_a)) / out_a;
                    let out_g = (blend_g * src_a + dst_g * dst_a * (1.0 - src_a)) / out_a;
                    let out_b = (blend_b * src_a + dst_b * dst_a * (1.0 - src_a)) / out_a;
                    
                    output[offset] = (out_r.clamp(0.0, 1.0) * 255.0) as u8;
                    output[offset + 1] = (out_g.clamp(0.0, 1.0) * 255.0) as u8;
                    output[offset + 2] = (out_b.clamp(0.0, 1.0) * 255.0) as u8;
                    output[offset + 3] = (out_a.clamp(0.0, 1.0) * 255.0) as u8;
                }
            }
        }
        
        Ok(())
    }
}