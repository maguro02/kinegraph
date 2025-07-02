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