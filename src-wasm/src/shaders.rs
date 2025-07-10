/// WebGPU用シェーダー定義

/// 頂点シェーダー
pub const VERTEX_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    // 正規化デバイス座標に変換 (-1.0 ~ 1.0)
    output.clip_position = vec4<f32>(input.position, 0.0, 1.0);
    output.color = input.color;
    return output;
}
"#;

/// フラグメントシェーダー
pub const FRAGMENT_SHADER: &str = r#"
struct FragmentInput {
    @location(0) color: vec4<f32>,
}

@fragment
fn fs_main(input: FragmentInput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

/// ブラシ用頂点シェーダー
pub const BRUSH_VERTEX_SHADER: &str = r#"
struct Uniforms {
    transform: mat4x4<f32>,
    brush_size: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = uniforms.transform * vec4<f32>(input.position, 0.0, 1.0);
    output.tex_coords = input.tex_coords;
    return output;
}
"#;

/// ブラシ用フラグメントシェーダー
pub const BRUSH_FRAGMENT_SHADER: &str = r#"
struct Uniforms {
    color: vec4<f32>,
}

@group(0) @binding(1)
var<uniform> uniforms: Uniforms;

@group(0) @binding(2)
var brush_texture: texture_2d<f32>;

@group(0) @binding(3)
var brush_sampler: sampler;

struct FragmentInput {
    @location(0) tex_coords: vec2<f32>,
}

@fragment
fn fs_main(input: FragmentInput) -> @location(0) vec4<f32> {
    let alpha = textureSample(brush_texture, brush_sampler, input.tex_coords).a;
    return vec4<f32>(uniforms.color.rgb, uniforms.color.a * alpha);
}
"#;

/// ストローク描画用シェーダー
pub const STROKE_SHADER: &str = r#"
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

/// レイヤー合成用シェーダー
pub const LAYER_COMPOSITE_SHADER: &str = r#"
struct LayerUniforms {
    opacity: f32,
    blend_mode: u32,
    _padding: vec2<f32>,
}

@group(0) @binding(0)
var layer_texture: texture_2d<f32>;
@group(0) @binding(1)
var layer_sampler: sampler;
@group(0) @binding(2)
var<uniform> layer_uniforms: LayerUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
}

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.tex_coord = tex_coord;
    return output;
}

// Blend mode functions
fn blend_normal(base: vec4<f32>, blend: vec4<f32>) -> vec4<f32> {
    return mix(base, blend, blend.a);
}

fn blend_multiply(base: vec4<f32>, blend: vec4<f32>) -> vec4<f32> {
    let result = base.rgb * blend.rgb;
    return vec4<f32>(mix(base.rgb, result, blend.a), base.a + blend.a * (1.0 - base.a));
}

fn blend_screen(base: vec4<f32>, blend: vec4<f32>) -> vec4<f32> {
    let result = vec3<f32>(1.0) - (vec3<f32>(1.0) - base.rgb) * (vec3<f32>(1.0) - blend.rgb);
    return vec4<f32>(mix(base.rgb, result, blend.a), base.a + blend.a * (1.0 - base.a));
}

fn blend_overlay(base: vec4<f32>, blend: vec4<f32>) -> vec4<f32> {
    var result: vec3<f32>;
    for (var i = 0; i < 3; i = i + 1) {
        if (base[i] < 0.5) {
            result[i] = 2.0 * base[i] * blend[i];
        } else {
            result[i] = 1.0 - 2.0 * (1.0 - base[i]) * (1.0 - blend[i]);
        }
    }
    return vec4<f32>(mix(base.rgb, result, blend.a), base.a + blend.a * (1.0 - base.a));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base_color = textureSample(layer_texture, layer_sampler, in.tex_coord);
    let layer_color = base_color * layer_uniforms.opacity;
    
    // Apply blend mode
    switch (layer_uniforms.blend_mode) {
        case 0u: { // Normal
            return blend_normal(base_color, layer_color);
        }
        case 1u: { // Multiply
            return blend_multiply(base_color, layer_color);
        }
        case 2u: { // Screen
            return blend_screen(base_color, layer_color);
        }
        case 3u: { // Overlay
            return blend_overlay(base_color, layer_color);
        }
        default: {
            return layer_color;
        }
    }
}
"#;