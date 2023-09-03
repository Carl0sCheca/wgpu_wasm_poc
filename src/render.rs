#[derive(Debug)]
pub struct Render {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub render_pipeline: std::rc::Rc<wgpu::RenderPipeline>,
    pub index_count: u32,
    pub transform_buffer: Option<std::rc::Rc<wgpu::Buffer>>,
    pub bind_groups: Vec<(u32, std::rc::Rc<wgpu::BindGroup>)>,
    pub instances: u32,
}

impl Render {
    // pub fn new<T: bytemuck::Pod>(
    //     device: &wgpu::Device,
    //     mesh: (Vec<T>, Vec<u16>),
    //     render_pipeline: std::rc::Rc<wgpu::RenderPipeline>,
    //     transform_buffer: Option<std::rc::Rc<wgpu::Buffer>>,
    //     bind_groups: Vec<(u32, std::rc::Rc<wgpu::BindGroup>)>
    // ) -> Self {
    //     let vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
    //         device,
    //         &wgpu::util::BufferInitDescriptor {
    //             label: Some("Vertex Buffer Init"),
    //             contents: bytemuck::cast_slice(&mesh.0),
    //             usage: wgpu::BufferUsages::VERTEX,
    //         },
    //     );

    //     let index_buffer = wgpu::util::DeviceExt::create_buffer_init(
    //         device,
    //         &wgpu::util::BufferInitDescriptor {
    //             label: Some("Index Buffer Init"),
    //             contents: bytemuck::cast_slice(&mesh.1),
    //             usage: wgpu::BufferUsages::INDEX,
    //         },
    //     );

    //     Self {
    //         vertex_buffer,
    //         index_buffer,
    //         render_pipeline,
    //         index_count: mesh.1.len() as u32,
    //         transform_buffer,
    //         bind_groups,
    //         instances: 1,
    //     }
    // }

    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);

        self.bind_groups.iter().for_each(|(id, bind_group)| {
            render_pass.set_bind_group(*id, bind_group, &[]);
        });

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        if let Some(buffer) = self.transform_buffer.as_ref() {
            render_pass.set_vertex_buffer(1, buffer.slice(..));
        }

        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.index_count, 0, 0..self.instances);
    }

    pub fn update_transform(&mut self, data: &[u8], queue: &mut wgpu::Queue) {
        if let Some(buffer) = &mut self.transform_buffer {
            queue.write_buffer(&buffer, 0, data);
        }
    }
}
