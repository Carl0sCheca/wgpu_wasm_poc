#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformsTexture {
    pub texture_index: u32,
    pub flip_x: i32,
    pub _padding: [f32; 6],
}

pub fn get_rect(size: nalgebra_glm::Vec3) -> (Vec<Vertex>, Vec<u16>) {
    (
        vec![
            Vertex {
                position: [size.x, size.y, 0.0],
                tex_coords: [1.0, 0.0],
            },
            Vertex {
                position: [-size.x, size.y, 0.0],
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [-size.x, -size.y, 0.0],
                tex_coords: [0.0, 1.0],
            },
            Vertex {
                position: [size.x, -size.y, 0.0],
                tex_coords: [1.0, 1.0],
            },
        ],
        vec![0, 1, 2, 2, 3, 0],
    )
}
