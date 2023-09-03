// Vertex shader

const OPENGL_TO_WGPU_MATRIX: mat4x4<f32> = mat4x4<f32>(
    vec4<f32>(1.0, 0.0, 0.0, 0.0),
    vec4<f32>(0.0, 1.0, 0.0, 0.0),
    vec4<f32>(0.0, 0.0, 0.5, 0.0),
    vec4<f32>(0.0, 0.0, 0.5, 1.0),
);

struct CameraUniform {
    projection: mat4x4<f32>,
    view: mat4x4<f32>,
}

@group(1) @binding(0)
var<uniform> camera: CameraUniform;


// struct UniformsTexture {
//     texture_index: u32,
//     flip_x: i32,
//     _padding: vec4<f32>,
// }

// @group(2) @binding(0)
// var<uniform> u_texture: UniformsTexture;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) tex_index: i32,
    @location(2) tex_flip_x: i32,
}

struct TransformInput {
    @location(5) matrix_0: vec4<f32>,
    @location(6) matrix_1: vec4<f32>,
    @location(7) matrix_2: vec4<f32>,
    @location(8) matrix_3: vec4<f32>,
    @location(9) index: i32,
    @location(10) tex_flip_x: i32,
};

@vertex
fn vs_main(
    model: VertexInput,
    transform: TransformInput
) -> VertexOutput {
    let transform_matrix = mat4x4<f32>(
        transform.matrix_0,
        transform.matrix_1,
        transform.matrix_2,
        transform.matrix_3,
    );

    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.tex_index = transform.index;
    out.tex_flip_x = transform.tex_flip_x;
    out.clip_position = OPENGL_TO_WGPU_MATRIX * camera.projection * camera.view * transform_matrix * vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let elementIndex = in.tex_index; // El índice del elemento que deseas dibujar
    let elementsPerRow = 10; // Número de elementos por fila
    let elementsPerColumn = 25; // Número de elementos por columna
    let elementColumn = elementIndex % elementsPerRow;
    let elementRow = elementIndex / elementsPerRow;

    let flip_x = select(in.tex_coords.x, 1.0 - in.tex_coords.x, in.tex_flip_x == 0);
    let tex_coords = vec2<f32>(
        (flip_x + f32(elementColumn)) / f32(elementsPerRow),
        (in.tex_coords.y + f32(elementRow)) / f32(elementsPerColumn)
    );

    var color = textureSample(t_diffuse, s_diffuse, tex_coords);
    if (color.a == 0.0) {
        discard;
    }

    return color;
}