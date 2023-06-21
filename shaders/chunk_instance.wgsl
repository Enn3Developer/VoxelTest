struct VertexInput {
    @location(0) position: u32,
    @location(1) tex_coords: vec2<f32>,
}

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
    ambient_strength: f32,
}

struct ChunkPos {
    chunk_pos: vec3<f32>
}

@group(0)@binding(0)
var<uniform> camera: CameraUniform;

@group(1)@binding(0)
var t_diffuse: texture_2d<f32>;
@group(1)@binding(1)
var s_diffuse: sampler;

@group(2)@binding(0)
var<uniform> chunk_pos: ChunkPos;

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    let scale = 0.5;

    let x = f32(model.position >> 6u);
    let y = f32((model.position >> 3u) & 7u);
    let z = f32(model.position & 7u);
    let position = vec3<f32>(x, y, z);

    let world_position = model_matrix * vec4<f32>((position + chunk_pos.chunk_pos) * scale, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_position;
    out.tex_coords = model.tex_coords;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let object_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);

    return object_color;
}