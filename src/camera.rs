#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    projection: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            projection: nalgebra_glm::Mat4::identity().into(),
            view: nalgebra_glm::Mat4::identity().into(),
        }
    }

    pub fn update(&mut self, projection: nalgebra_glm::Mat4, view: nalgebra_glm::Mat4) {
        self.projection = projection.into();
        self.view = view.into();
    }
}
