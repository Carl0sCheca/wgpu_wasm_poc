mod camera;
mod map;
mod render;
mod texture;
mod transform;
mod vertex;

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use wasm_timer::Instant;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use wgpu::util::DeviceExt;

use winit::window::Window;

#[derive(Debug)]
struct Animation {
    index: i32,
    offset: i32,
    max: i32,
    speed: Duration,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Could't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
        .build(&event_loop)
        .unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(1280, 720));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

    let mut state = State::new(window).await;

    let mut previous_frame = Instant::now();

    const FPS: u64 = 61;

    cfg_if::cfg_if! {
        if #[cfg(not(target_arch = "wasm32"))] {
            let frame_duration = Duration::from_secs(1) / FPS as _;
        }
    }

    event_loop.run(move |event, _, control_flow| match event {
        Event::RedrawRequested(window_id) if window_id == state.window().id() => {
            let current_frame = Instant::now();
            let delta_time = current_frame - previous_frame;
            previous_frame = current_frame;
            state.update(delta_time);
            match state.render() {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(e) => eprintln!("{:?}", e),
            }
            cfg_if::cfg_if! {
                if #[cfg(not(target_arch = "wasm32"))] {
                    let frame_end = Instant::now();
                    let elapsed = frame_end - current_frame;
                    if elapsed < frame_duration {
                        std::thread::sleep(frame_duration - elapsed);
                    }
                }
            }
        }
        Event::MainEventsCleared => {
            state.window().request_redraw();
        }
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == state.window().id() => {
            if !state.input(event) {
                match event {
                    WindowEvent::CursorMoved { position, .. } => {
                        state.mouse_pos =
                            nalgebra_glm::vec3(position.x as f32, position.y as f32, 0.0);
                    }
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(key),
                                ..
                            },
                        ..
                    } => match key {
                        VirtualKeyCode::W => {
                            // state.transform.position.y += 0.1;
                            state.actions[0] = true;
                        }
                        VirtualKeyCode::S => {
                            // state.transform.position.y -= 0.1;
                            state.actions[2] = true;
                        }
                        VirtualKeyCode::A => {
                            // state.transform.position.x -= 0.1;
                            state.actions[1] = true;
                        }
                        VirtualKeyCode::D => {
                            // state.transform.position.x += 0.1;
                            state.actions[3] = true;
                        }
                        VirtualKeyCode::Q => {
                            // state.transform.rotation.z -= 1.0;
                            state.actions[4] = true;
                        }
                        VirtualKeyCode::E => {
                            // state.transform.rotation.z += 1.0;
                            state.actions[5] = true;
                        }
                        VirtualKeyCode::R => {
                            state.zoom = if state.zoom > 0.0 {
                                state.zoom - 0.1
                            } else {
                                0.0
                            }
                        }
                        VirtualKeyCode::F => {
                            state.zoom += 0.1;
                        }
                        VirtualKeyCode::LShift => state.actions[6] = true,
                        _ => {}
                    },
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Released,
                                virtual_keycode: Some(key),
                                ..
                            },
                        ..
                    } => match key {
                        VirtualKeyCode::W => {
                            state.actions[0] = false;
                        }
                        VirtualKeyCode::S => {
                            state.actions[2] = false;
                        }
                        VirtualKeyCode::A => {
                            state.actions[1] = false;
                        }
                        VirtualKeyCode::D => {
                            state.actions[3] = false;
                        }
                        VirtualKeyCode::Q => {
                            state.actions[4] = false;
                        }
                        VirtualKeyCode::E => {
                            state.actions[5] = false;
                        }
                        VirtualKeyCode::LShift => state.actions[6] = false,
                        _ => {}
                    },
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    });
}

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Window,
    // render_pipeline: wgpu::RenderPipeline,
    // vertex_buffer: wgpu::Buffer,
    // index_buffer: wgpu::Buffer,
    // diffuse_bind_group: wgpu::BindGroup,
    // uniform_bind_group: wgpu::BindGroup,
    // uniforms_texture: vertex::UniformsTexture,
    // uniform_buffer: wgpu::Buffer,
    time_since_last_frame: Duration,
    camera_buffer: std::rc::Rc<wgpu::Buffer>,
    // camera_bind_group: std::rc::Rc<wgpu::BindGroup>,
    camera: (nalgebra_glm::Mat4, nalgebra_glm::Mat4),
    camera_uniform: camera::CameraUniform,
    // transform_buffer: wgpu::Buffer,
    transform: std::rc::Rc<std::cell::RefCell<transform::Transform>>,
    instances: Vec<std::rc::Rc<std::cell::RefCell<transform::Transform>>>,
    // instance_buffer: wgpu::Buffer,
    // instances_vertex_buffer: wgpu::Buffer,
    animation: Animation,
    actions: Vec<bool>,
    zoom: f32,
    // indices: Vec<u16>,
    mouse_pos: nalgebra_glm::Vec3,
    renders: Vec<render::Render>,
}

impl State {
    async fn new(window: Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps.formats[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        // let uniforms_texture = vertex::UniformsTexture {
        //     texture_index: 1,
        //     flip_x: 0,
        //     _padding: [0.0; 6],
        // };
        // let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: Some("Uniform Buffer"),
        //     contents: bytemuck::cast_slice(&[uniforms_texture]),
        //     usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        // });
        // let uniform_bind_group_layout =
        //     device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //         entries: &[wgpu::BindGroupLayoutEntry {
        //             binding: 0,
        //             visibility: wgpu::ShaderStages::FRAGMENT,
        //             ty: wgpu::BindingType::Buffer {
        //                 ty: wgpu::BufferBindingType::Uniform,
        //                 has_dynamic_offset: false,
        //                 min_binding_size: None,
        //             },
        //             count: None,
        //         }],
        //         label: Some("uniform_bind_group_layout"),
        //     });
        // let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     layout: &uniform_bind_group_layout,
        //     entries: &[wgpu::BindGroupEntry {
        //         binding: 0,
        //         resource: uniform_buffer.as_entire_binding(),
        //     }],
        //     label: Some("uniform_bind_group"),
        // });

        let mut camera_uniform = camera::CameraUniform::new();

        let camera_buffer = std::rc::Rc::new(wgpu::util::DeviceExt::create_buffer_init(
            &device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            },
        ));

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group =
            std::rc::Rc::new(device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &camera_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }],
                label: Some("Camera Bind Group"),
            }));

        let transform = {
            let mut t = transform::Transform::new();
            t.label = Some("transform".to_string());
            t.translate(&nalgebra_glm::vec3(400.0, 400.0, 0.0));
            // t.rotate(&nalgebra_glm::vec3(0.0, 0.0, -45.0));
            std::rc::Rc::new(std::cell::RefCell::new(t))
        };

        let instances = (0..10)
            .flat_map(|y: i32| {
                (0..10).map(move |x| {
                    let mut t = transform::Transform::new();
                    t.translate(&nalgebra_glm::vec3(
                        32.0 + x as f32 * 32.0,
                        32.0 + y as f32 * 32.0,
                        0.0,
                    ));
                    t.label = Some(format!("{}", x + (y * 10)).to_string());
                    // t.rotate(&nalgebra_glm::vec3(0.0, 0.0, 45.0));
                    std::rc::Rc::new(std::cell::RefCell::new(t))
                })
            })
            .collect::<Vec<_>>();

        let map = map::load_map("./resources/mapa.json").await;

        let renders = vec![
            // MAP LAYER 0
            {
                let diffuse_bytes = &map.tileset.image;
                let diffuse_texture = texture::Texture::from_bytes(
                    &device,
                    &queue,
                    &diffuse_bytes,
                    "fullspritesheet.png",
                )
                .unwrap();
                let texture_bind_group_layout: wgpu::BindGroupLayout = device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    multisampled: false,
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                                count: None,
                            },
                        ],
                        label: Some("texture_bind_group_layout"),
                    });
                let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                        },
                    ],
                    label: Some("diffuse_bind_group"),
                });

                let map_size = map.size.clone();

                let mut map_data = vec![];
                if let map::Layers::TileLayer { data, .. } = &map.layers[0] {
                    for i in (0..(map_size.0 * map_size.1) as usize)
                        .step_by(map_size.0 as usize)
                        .rev()
                    {
                        map_data.extend_from_slice(&data[i..(i as u32 + map_size.0) as usize])
                    }
                }

                let instances = (0..map_size.0 as i32)
                    .flat_map(|x: i32| {
                        let map_data = map_data.clone();
                        (0..map_size.1 as i32).map(move |y| {
                            let mut t = transform::Transform::new();
                            t.translate(&nalgebra_glm::vec3(
                                x as f32 * map.tile_size.0 as f32 * 2.0,
                                y as f32 * map.tile_size.1 as f32 * 2.0,
                                0.0,
                            ));
                            t.label = Some(format!("{}", x + (y * map_size.1 as i32)).to_string());
                            if map_data.len() > 0 {
                                t.index = map_data[(x + (y * map_size.0 as i32)) as usize];
                                t.flip_x = 1;
                            }
                            std::rc::Rc::new(std::cell::RefCell::new(t))
                        })
                    })
                    .collect::<Vec<_>>();

                let instance_data = instances
                    .iter()
                    .map(|x| x.as_ref().borrow().to_raw())
                    .collect::<Vec<_>>();

                let instance_buffer =
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Instance Buffer"),
                        contents: bytemuck::cast_slice(&instance_data),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    });

                let (vertex_points, vertex_indices) =
                    vertex::get_rect(nalgebra_glm::vec3(16.0, 16.0, 0.0));
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&vertex_points),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&vertex_indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

                let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                        "shaders/tile.wgsl"
                    ))),
                });

                let render_pipeline_layout =
                    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Render Pipeline Layout"),
                        bind_group_layouts: &[
                            &texture_bind_group_layout,
                            // &uniform_bind_group_layout,
                            &camera_bind_group_layout,
                        ],
                        push_constant_ranges: &[],
                    });

                let render_pipeline = std::rc::Rc::new(device.create_render_pipeline(
                    &wgpu::RenderPipelineDescriptor {
                        label: None,
                        layout: Some(&render_pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &shader,
                            entry_point: "vs_main",
                            buffers: &[
                                wgpu::VertexBufferLayout {
                                    array_stride: std::mem::size_of::<vertex::Vertex>()
                                        as wgpu::BufferAddress,
                                    step_mode: wgpu::VertexStepMode::Vertex,
                                    attributes: &[
                                        wgpu::VertexAttribute {
                                            offset: 0,
                                            shader_location: 0,
                                            format: wgpu::VertexFormat::Float32x3,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 3]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 1,
                                            format: wgpu::VertexFormat::Float32x2,
                                        },
                                    ],
                                },
                                wgpu::VertexBufferLayout {
                                    array_stride: std::mem::size_of::<transform::TransformRaw>()
                                        as wgpu::BufferAddress,
                                    step_mode: wgpu::VertexStepMode::Instance,
                                    attributes: &[
                                        wgpu::VertexAttribute {
                                            offset: 0,
                                            shader_location: 5,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 4]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 6,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 8]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 7,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 12]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 8,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 16]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 9,
                                            format: wgpu::VertexFormat::Sint32,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 17]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 10,
                                            format: wgpu::VertexFormat::Sint32,
                                        },
                                    ],
                                },
                            ],
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: &shader,
                            entry_point: "fs_main",
                            targets: &[Some(wgpu::ColorTargetState {
                                format: surface_format,
                                blend: Some(wgpu::BlendState {
                                    color: wgpu::BlendComponent::REPLACE,
                                    alpha: wgpu::BlendComponent::OVER,
                                }),
                                write_mask: wgpu::ColorWrites::all(),
                            })],
                        }),
                        primitive: wgpu::PrimitiveState {
                            topology: wgpu::PrimitiveTopology::TriangleList,
                            strip_index_format: None,
                            front_face: wgpu::FrontFace::Ccw,
                            cull_mode: Some(wgpu::Face::Back),
                            unclipped_depth: false,
                            polygon_mode: wgpu::PolygonMode::Fill,
                            conservative: false,
                        },
                        depth_stencil: None,
                        multisample: wgpu::MultisampleState {
                            count: 1,
                            mask: !0,
                            alpha_to_coverage_enabled: false,
                        },
                        multiview: None,
                    },
                ));

                render::Render {
                    vertex_buffer,
                    index_buffer,
                    render_pipeline,
                    index_count: vertex_indices.len() as _,
                    transform_buffer: Some(std::rc::Rc::new(instance_buffer)),
                    bind_groups: vec![
                        (0, std::rc::Rc::new(texture_bind_group)),
                        (1, camera_bind_group.clone()),
                    ],
                    instances: instances.len() as u32,
                }
            },
            // MAP LAYER 1
            {
                let diffuse_bytes = &map.tileset.image;
                let diffuse_texture = texture::Texture::from_bytes(
                    &device,
                    &queue,
                    &diffuse_bytes,
                    "fullspritesheet.png",
                )
                .unwrap();
                let texture_bind_group_layout: wgpu::BindGroupLayout = device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    multisampled: false,
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                                count: None,
                            },
                        ],
                        label: Some("texture_bind_group_layout"),
                    });
                let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                        },
                    ],
                    label: Some("diffuse_bind_group"),
                });

                let map_size = map.size.clone();

                let mut map_data = vec![];
                if let map::Layers::TileLayer { data, .. } = &map.layers[1] {
                    for i in (0..(map_size.0 * map_size.1) as usize)
                        .step_by(map_size.0 as usize)
                        .rev()
                    {
                        map_data.extend_from_slice(&data[i..(i as u32 + map_size.0) as usize])
                    }
                }

                let instances = (0..map_size.0 as i32)
                    .flat_map(|x: i32| {
                        let map_data = map_data.clone();
                        (0..map_size.1 as i32).map(move |y| {
                            let mut t = transform::Transform::new();
                            t.translate(&nalgebra_glm::vec3(
                                x as f32 * map.tile_size.0 as f32 * 2.0,
                                y as f32 * map.tile_size.1 as f32 * 2.0,
                                0.0,
                            ));
                            t.label = Some(format!("{}", x + (y * map_size.1 as i32)).to_string());
                            if map_data.len() > 0 {
                                t.index = map_data[(x + (y * map_size.0 as i32)) as usize];
                                t.flip_x = 1;
                            }
                            std::rc::Rc::new(std::cell::RefCell::new(t))
                        })
                    })
                    .collect::<Vec<_>>();

                let instance_data = instances
                    .iter()
                    .map(|x| x.as_ref().borrow().to_raw())
                    .collect::<Vec<_>>();

                let instance_buffer =
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Instance Buffer"),
                        contents: bytemuck::cast_slice(&instance_data),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    });

                let (vertex_points, vertex_indices) =
                    vertex::get_rect(nalgebra_glm::vec3(16.0, 16.0, 0.0));
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&vertex_points),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&vertex_indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

                let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                        "shaders/tile.wgsl"
                    ))),
                });

                let render_pipeline_layout =
                    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Render Pipeline Layout"),
                        bind_group_layouts: &[
                            &texture_bind_group_layout,
                            // &uniform_bind_group_layout,
                            &camera_bind_group_layout,
                        ],
                        push_constant_ranges: &[],
                    });

                let render_pipeline = std::rc::Rc::new(device.create_render_pipeline(
                    &wgpu::RenderPipelineDescriptor {
                        label: None,
                        layout: Some(&render_pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &shader,
                            entry_point: "vs_main",
                            buffers: &[
                                wgpu::VertexBufferLayout {
                                    array_stride: std::mem::size_of::<vertex::Vertex>()
                                        as wgpu::BufferAddress,
                                    step_mode: wgpu::VertexStepMode::Vertex,
                                    attributes: &[
                                        wgpu::VertexAttribute {
                                            offset: 0,
                                            shader_location: 0,
                                            format: wgpu::VertexFormat::Float32x3,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 3]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 1,
                                            format: wgpu::VertexFormat::Float32x2,
                                        },
                                    ],
                                },
                                wgpu::VertexBufferLayout {
                                    array_stride: std::mem::size_of::<transform::TransformRaw>()
                                        as wgpu::BufferAddress,
                                    step_mode: wgpu::VertexStepMode::Instance,
                                    attributes: &[
                                        wgpu::VertexAttribute {
                                            offset: 0,
                                            shader_location: 5,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 4]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 6,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 8]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 7,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 12]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 8,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 16]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 9,
                                            format: wgpu::VertexFormat::Sint32,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 17]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 10,
                                            format: wgpu::VertexFormat::Sint32,
                                        },
                                    ],
                                },
                            ],
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: &shader,
                            entry_point: "fs_main",
                            targets: &[Some(wgpu::ColorTargetState {
                                format: surface_format,
                                blend: Some(wgpu::BlendState {
                                    color: wgpu::BlendComponent::REPLACE,
                                    alpha: wgpu::BlendComponent::OVER,
                                }),
                                write_mask: wgpu::ColorWrites::all(),
                            })],
                        }),
                        primitive: wgpu::PrimitiveState {
                            topology: wgpu::PrimitiveTopology::TriangleList,
                            strip_index_format: None,
                            front_face: wgpu::FrontFace::Ccw,
                            cull_mode: Some(wgpu::Face::Back),
                            unclipped_depth: false,
                            polygon_mode: wgpu::PolygonMode::Fill,
                            conservative: false,
                        },
                        depth_stencil: None,
                        multisample: wgpu::MultisampleState {
                            count: 1,
                            mask: !0,
                            alpha_to_coverage_enabled: false,
                        },
                        multiview: None,
                    },
                ));

                render::Render {
                    vertex_buffer,
                    index_buffer,
                    render_pipeline,
                    index_count: vertex_indices.len() as _,
                    transform_buffer: Some(std::rc::Rc::new(instance_buffer)),
                    bind_groups: vec![
                        (0, std::rc::Rc::new(texture_bind_group)),
                        (1, camera_bind_group.clone()),
                    ],
                    instances: instances.len() as u32,
                }
            },
            // MAP LAYER 2
            {
                let diffuse_bytes = &map.tileset.image;
                let diffuse_texture = texture::Texture::from_bytes(
                    &device,
                    &queue,
                    &diffuse_bytes,
                    "fullspritesheet.png",
                )
                .unwrap();
                let texture_bind_group_layout: wgpu::BindGroupLayout = device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    multisampled: false,
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                                count: None,
                            },
                        ],
                        label: Some("texture_bind_group_layout"),
                    });
                let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                        },
                    ],
                    label: Some("diffuse_bind_group"),
                });

                let map_size = map.size.clone();

                let mut map_data = vec![];
                if let map::Layers::TileLayer { data, .. } = &map.layers[2] {
                    for i in (0..(map_size.0 * map_size.1) as usize)
                        .step_by(map_size.0 as usize)
                        .rev()
                    {
                        map_data.extend_from_slice(&data[i..(i as u32 + map_size.0) as usize])
                    }
                }

                let instances = (0..map_size.0 as i32)
                    .flat_map(|x: i32| {
                        let map_data = map_data.clone();
                        (0..map_size.1 as i32).map(move |y| {
                            let mut t = transform::Transform::new();
                            t.translate(&nalgebra_glm::vec3(
                                x as f32 * map.tile_size.0 as f32 * 2.0,
                                y as f32 * map.tile_size.1 as f32 * 2.0,
                                0.0,
                            ));
                            t.label = Some(format!("{}", x + (y * map_size.1 as i32)).to_string());
                            if map_data.len() > 0 {
                                t.index = map_data[(x + (y * map_size.0 as i32)) as usize];
                                t.flip_x = 1;
                            }
                            std::rc::Rc::new(std::cell::RefCell::new(t))
                        })
                    })
                    .collect::<Vec<_>>();

                let instance_data = instances
                    .iter()
                    .map(|x| x.as_ref().borrow().to_raw())
                    .collect::<Vec<_>>();

                let instance_buffer =
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Instance Buffer"),
                        contents: bytemuck::cast_slice(&instance_data),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    });

                let (vertex_points, vertex_indices) =
                    vertex::get_rect(nalgebra_glm::vec3(16.0, 16.0, 0.0));
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&vertex_points),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&vertex_indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

                let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                        "shaders/tile.wgsl"
                    ))),
                });

                let render_pipeline_layout =
                    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Render Pipeline Layout"),
                        bind_group_layouts: &[
                            &texture_bind_group_layout,
                            // &uniform_bind_group_layout,
                            &camera_bind_group_layout,
                        ],
                        push_constant_ranges: &[],
                    });

                let render_pipeline = std::rc::Rc::new(device.create_render_pipeline(
                    &wgpu::RenderPipelineDescriptor {
                        label: None,
                        layout: Some(&render_pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &shader,
                            entry_point: "vs_main",
                            buffers: &[
                                wgpu::VertexBufferLayout {
                                    array_stride: std::mem::size_of::<vertex::Vertex>()
                                        as wgpu::BufferAddress,
                                    step_mode: wgpu::VertexStepMode::Vertex,
                                    attributes: &[
                                        wgpu::VertexAttribute {
                                            offset: 0,
                                            shader_location: 0,
                                            format: wgpu::VertexFormat::Float32x3,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 3]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 1,
                                            format: wgpu::VertexFormat::Float32x2,
                                        },
                                    ],
                                },
                                wgpu::VertexBufferLayout {
                                    array_stride: std::mem::size_of::<transform::TransformRaw>()
                                        as wgpu::BufferAddress,
                                    step_mode: wgpu::VertexStepMode::Instance,
                                    attributes: &[
                                        wgpu::VertexAttribute {
                                            offset: 0,
                                            shader_location: 5,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 4]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 6,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 8]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 7,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 12]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 8,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 16]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 9,
                                            format: wgpu::VertexFormat::Sint32,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 17]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 10,
                                            format: wgpu::VertexFormat::Sint32,
                                        },
                                    ],
                                },
                            ],
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: &shader,
                            entry_point: "fs_main",
                            targets: &[Some(wgpu::ColorTargetState {
                                format: surface_format,
                                blend: Some(wgpu::BlendState {
                                    color: wgpu::BlendComponent::REPLACE,
                                    alpha: wgpu::BlendComponent::OVER,
                                }),
                                write_mask: wgpu::ColorWrites::all(),
                            })],
                        }),
                        primitive: wgpu::PrimitiveState {
                            topology: wgpu::PrimitiveTopology::TriangleList,
                            strip_index_format: None,
                            front_face: wgpu::FrontFace::Ccw,
                            cull_mode: Some(wgpu::Face::Back),
                            unclipped_depth: false,
                            polygon_mode: wgpu::PolygonMode::Fill,
                            conservative: false,
                        },
                        depth_stencil: None,
                        multisample: wgpu::MultisampleState {
                            count: 1,
                            mask: !0,
                            alpha_to_coverage_enabled: false,
                        },
                        multiview: None,
                    },
                ));

                render::Render {
                    vertex_buffer,
                    index_buffer,
                    render_pipeline,
                    index_count: vertex_indices.len() as _,
                    transform_buffer: Some(std::rc::Rc::new(instance_buffer)),
                    bind_groups: vec![
                        (0, std::rc::Rc::new(texture_bind_group)),
                        (1, camera_bind_group.clone()),
                    ],
                    instances: instances.len() as u32,
                }
            },
            // TRANSFORM
            {
                let diffuse_bytes = include_bytes!("../resources/fullspritesheet.png");

                let diffuse_texture = texture::Texture::from_bytes(
                    &device,
                    &queue,
                    diffuse_bytes,
                    "fullspritesheet.png",
                )
                .unwrap();
                let texture_bind_group_layout: wgpu::BindGroupLayout = device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    multisampled: false,
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                                count: None,
                            },
                        ],
                        label: Some("texture_bind_group_layout"),
                    });
                let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                        },
                    ],
                    label: Some("diffuse_bind_group"),
                });

                let (vertex_points, vertex_indices) =
                    vertex::get_rect(nalgebra_glm::vec3(96.0, 64.0, 0.0));
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&vertex_points),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&vertex_indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

                let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                        "shaders/main.wgsl"
                    ))),
                });

                let render_pipeline_layout =
                    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Render Pipeline Layout"),
                        bind_group_layouts: &[
                            &texture_bind_group_layout,
                            // &uniform_bind_group_layout,
                            &camera_bind_group_layout,
                        ],
                        push_constant_ranges: &[],
                    });

                let render_pipeline = std::rc::Rc::new(device.create_render_pipeline(
                    &wgpu::RenderPipelineDescriptor {
                        label: None,
                        layout: Some(&render_pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &shader,
                            entry_point: "vs_main",
                            buffers: &[
                                wgpu::VertexBufferLayout {
                                    array_stride: std::mem::size_of::<vertex::Vertex>()
                                        as wgpu::BufferAddress,
                                    step_mode: wgpu::VertexStepMode::Vertex,
                                    attributes: &[
                                        wgpu::VertexAttribute {
                                            offset: 0,
                                            shader_location: 0,
                                            format: wgpu::VertexFormat::Float32x3,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 3]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 1,
                                            format: wgpu::VertexFormat::Float32x2,
                                        },
                                    ],
                                },
                                wgpu::VertexBufferLayout {
                                    array_stride: std::mem::size_of::<transform::TransformRaw>()
                                        as wgpu::BufferAddress,
                                    step_mode: wgpu::VertexStepMode::Instance,
                                    attributes: &[
                                        wgpu::VertexAttribute {
                                            offset: 0,
                                            shader_location: 5,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 4]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 6,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 8]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 7,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 12]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 8,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 16]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 9,
                                            format: wgpu::VertexFormat::Sint32,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 17]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 10,
                                            format: wgpu::VertexFormat::Sint32,
                                        },
                                    ],
                                },
                            ],
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: &shader,
                            entry_point: "fs_main",
                            targets: &[Some(wgpu::ColorTargetState {
                                format: surface_format,
                                blend: Some(wgpu::BlendState {
                                    color: wgpu::BlendComponent::REPLACE,
                                    alpha: wgpu::BlendComponent::OVER,
                                }),
                                write_mask: wgpu::ColorWrites::all(),
                            })],
                        }),
                        primitive: wgpu::PrimitiveState {
                            topology: wgpu::PrimitiveTopology::TriangleList,
                            strip_index_format: None,
                            front_face: wgpu::FrontFace::Ccw,
                            cull_mode: Some(wgpu::Face::Back),
                            unclipped_depth: false,
                            polygon_mode: wgpu::PolygonMode::Fill,
                            conservative: false,
                        },
                        depth_stencil: None,
                        multisample: wgpu::MultisampleState {
                            count: 1,
                            mask: !0,
                            alpha_to_coverage_enabled: false,
                        },
                        multiview: None,
                    },
                ));

                render::Render {
                    vertex_buffer,
                    index_buffer,
                    render_pipeline,
                    index_count: vertex_indices.len() as _,
                    transform_buffer: Some(std::rc::Rc::new(device.create_buffer_init(
                        &wgpu::util::BufferInitDescriptor {
                            label: Some("Transform Buffer"),
                            contents: bytemuck::cast_slice(&[transform.as_ref().borrow().to_raw()]),
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        },
                    ))),
                    bind_groups: vec![
                        (0, std::rc::Rc::new(texture_bind_group)),
                        (1, camera_bind_group.clone()),
                    ],
                    instances: 1,
                }
            },
            // INSTANCES
            {
                let diffuse_bytes = include_bytes!("../resources/fullspritesheet.png");
                let diffuse_texture = texture::Texture::from_bytes(
                    &device,
                    &queue,
                    diffuse_bytes,
                    "fullspritesheet.png",
                )
                .unwrap();
                let texture_bind_group_layout: wgpu::BindGroupLayout = device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    multisampled: false,
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                                count: None,
                            },
                        ],
                        label: Some("texture_bind_group_layout"),
                    });
                let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &texture_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                        },
                    ],
                    label: Some("diffuse_bind_group"),
                });

                let instance_data = instances
                    .iter()
                    .map(|x| x.as_ref().borrow().to_raw())
                    .collect::<Vec<_>>();

                let instance_buffer =
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Instance Buffer"),
                        contents: bytemuck::cast_slice(&instance_data),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    });

                let (vertex_points, vertex_indices) =
                    vertex::get_rect(nalgebra_glm::vec3(48.0, 32.0, 0.0));
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&vertex_points),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&vertex_indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

                let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                        "shaders/main.wgsl"
                    ))),
                });

                let render_pipeline_layout =
                    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Render Pipeline Layout"),
                        bind_group_layouts: &[
                            &texture_bind_group_layout,
                            // &uniform_bind_group_layout,
                            &camera_bind_group_layout,
                        ],
                        push_constant_ranges: &[],
                    });

                let render_pipeline = std::rc::Rc::new(device.create_render_pipeline(
                    &wgpu::RenderPipelineDescriptor {
                        label: None,
                        layout: Some(&render_pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &shader,
                            entry_point: "vs_main",
                            buffers: &[
                                wgpu::VertexBufferLayout {
                                    array_stride: std::mem::size_of::<vertex::Vertex>()
                                        as wgpu::BufferAddress,
                                    step_mode: wgpu::VertexStepMode::Vertex,
                                    attributes: &[
                                        wgpu::VertexAttribute {
                                            offset: 0,
                                            shader_location: 0,
                                            format: wgpu::VertexFormat::Float32x3,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 3]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 1,
                                            format: wgpu::VertexFormat::Float32x2,
                                        },
                                    ],
                                },
                                wgpu::VertexBufferLayout {
                                    array_stride: std::mem::size_of::<transform::TransformRaw>()
                                        as wgpu::BufferAddress,
                                    step_mode: wgpu::VertexStepMode::Instance,
                                    attributes: &[
                                        wgpu::VertexAttribute {
                                            offset: 0,
                                            shader_location: 5,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 4]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 6,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 8]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 7,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 12]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 8,
                                            format: wgpu::VertexFormat::Float32x4,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 16]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 9,
                                            format: wgpu::VertexFormat::Sint32,
                                        },
                                        wgpu::VertexAttribute {
                                            offset: std::mem::size_of::<[f32; 17]>()
                                                as wgpu::BufferAddress,
                                            shader_location: 10,
                                            format: wgpu::VertexFormat::Sint32,
                                        },
                                    ],
                                },
                            ],
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: &shader,
                            entry_point: "fs_main",
                            targets: &[Some(wgpu::ColorTargetState {
                                format: surface_format,
                                blend: Some(wgpu::BlendState {
                                    color: wgpu::BlendComponent::REPLACE,
                                    alpha: wgpu::BlendComponent::OVER,
                                }),
                                write_mask: wgpu::ColorWrites::all(),
                            })],
                        }),
                        primitive: wgpu::PrimitiveState {
                            topology: wgpu::PrimitiveTopology::TriangleList,
                            strip_index_format: None,
                            front_face: wgpu::FrontFace::Ccw,
                            cull_mode: Some(wgpu::Face::Back),
                            unclipped_depth: false,
                            polygon_mode: wgpu::PolygonMode::Fill,
                            conservative: false,
                        },
                        depth_stencil: None,
                        multisample: wgpu::MultisampleState {
                            count: 1,
                            mask: !0,
                            alpha_to_coverage_enabled: false,
                        },
                        multiview: None,
                    },
                ));

                render::Render {
                    vertex_buffer,
                    index_buffer,
                    render_pipeline,
                    index_count: vertex_indices.len() as _,
                    transform_buffer: Some(std::rc::Rc::new(instance_buffer)),
                    bind_groups: vec![
                        (0, std::rc::Rc::new(texture_bind_group)),
                        (1, camera_bind_group.clone()),
                    ],
                    instances: instances.len() as u32,
                }
            },
        ];

        // let transform_center =
        //     std::rc::Rc::new(std::cell::RefCell::new(transform::Transform::new()));
        // transform_center.as_ref().borrow_mut().label = Some("transform center".to_string());
        // transform::Transform::parent(transform_center.clone(), instances[98].clone());
        // transform::Transform::parent(transform.clone(), transform_center.clone());
        // transform::Transform::parent(instances[98].clone(), instances[97].clone());
        // transform::Transform::parent(instances[97].clone(), instances[96].clone());
        // transform::Transform::parent(transform_center.clone(), instances[99].clone());

        // instances[99]
        //     .as_ref()
        //     .borrow_mut()
        //     .scale(&nalgebra_glm::vec3(2.0, 2.0, 2.0));
        // instances[99].as_ref().borrow_mut().index = 60;

        // dbg!(&transform);

        let camera = (
            nalgebra_glm::ortho_lh(
                0.0,
                window.inner_size().width as f32,
                0.0,
                window.inner_size().height as f32,
                0.025,
                1000.0,
            ),
            // nalgebra_glm::perspective_lh(1280.0 / 720.0, 45.0, 0.1, 1000.0),
            nalgebra_glm::Mat4::look_at_lh(
                &nalgebra_glm::Vec3::new(0.0, 0.0, -1.0).into(),
                &nalgebra_glm::Vec3::new(0.0, 0.0, 0.0).into(),
                &nalgebra_glm::Vec3::new(0.0, 1.0, 0.0),
            ),
        );

        camera_uniform.update(camera.0, camera.1);
        queue.write_buffer(&camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));

        let animation = Animation {
            index: 0,
            offset: 190,
            max: 8,
            speed: Duration::from_millis(1000 / 15),
        };

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            // render_pipeline,
            // vertex_buffer,
            // index_buffer,
            // diffuse_bind_group: texture_bind_group,
            // uniform_bind_group,
            // uniform_buffer,
            // uniforms_texture,
            time_since_last_frame: Duration::from_millis(1000 / 15),
            camera_buffer,
            // camera_bind_group,
            camera,
            camera_uniform,
            // transform_buffer,
            transform,
            animation,
            actions: vec![false, false, false, false, false, false, false],
            // instances,
            // instance_buffer,
            // instances_vertex_buffer,
            zoom: 1.0,
            // indices: vertex_indices,
            mouse_pos: nalgebra_glm::vec3(0.0, 0.0, 0.0),
            renders,
            instances,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, _event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self, delta_time: Duration) {
        let dt = delta_time.as_nanos() as f64 / 1_000_000_000.0;
        {
            let mut direction = nalgebra_glm::Vec3::zeros();
            if self.actions[0] {
                direction += nalgebra_glm::vec3(0.0, 1.0, 0.0);
            } else if self.actions[2] {
                direction -= nalgebra_glm::vec3(0.0, 1.0, 0.0);
            }

            let mut transform = self.transform.as_ref().borrow_mut();

            if self.actions[1] {
                // direction -= nalgebra_glm::vec3(1.0, 0.0, 0.0);
                direction += -transform.left();
                transform.flip_x = 0;
            } else if self.actions[3] {
                // direction += nalgebra_glm::vec3(1.0, 0.0, 0.0);
                direction += transform.left();
                transform.flip_x = 1;
            }

            // if self.actions[4] {
            //     //q
            //     let actual_rotation = transform.rotation.clone();
            //     transform.rotate(
            //         &(actual_rotation + nalgebra_glm::vec3(0.0, 0.0, 1.0) * dt as f32 * 200.0),
            //     );
            //     // let actual_rotation = self.instances[98].as_ref().borrow().rotation.clone();
            //     // self.instances[98].as_ref().borrow_mut().rotate(
            //     //     &(actual_rotation + nalgebra_glm::vec3(0.0, 0.0, 1.0) * dt as f32 * 200.0),
            //     // );
            // } else if self.actions[5] {
            //     //e
            //     let actual_rotation = transform.rotation.clone();
            //     transform.rotate(
            //         &(actual_rotation + nalgebra_glm::vec3(0.0, 0.0, -1.0) * dt as f32 * 200.0),
            //     );
            //     // let actual_rotation = self.instances[98].as_ref().borrow().rotation.clone();
            //     // self.instances[98].as_ref().borrow_mut().rotate(
            //     //     &(actual_rotation - nalgebra_glm::vec3(0.0, 0.0, 1.0) * dt as f32 * 200.0),
            //     // );
            // }

            let mouse_pos = self.mouse_pos.clone();
            let screen_pos = nalgebra_glm::vec4(
                (mouse_pos.x - self.size.width as f32 * 0.5) * self.zoom,
                (mouse_pos.y - self.size.height as f32 * 0.5) * self.zoom,
                0.0,
                1.0,
            );

            let camera_inverse = nalgebra_glm::inverse(&self.camera.1);
            let world_pos = camera_inverse * screen_pos;
            let mouse_pos = world_pos.xyz();
            let mouse_pos = nalgebra_glm::vec3(mouse_pos.x, -mouse_pos.y, 0.0);
            let diff = nalgebra_glm::normalize(&(mouse_pos - nalgebra_glm::vec3(0.0, 0.0, 0.0)));
            let angle = diff.y.atan2(diff.x);
            transform.rotate(&nalgebra_glm::vec3(0.0, 0.0, angle.to_degrees()));

            let transform_position = transform.position.clone();
            let running = if self.actions[6] { 4.0 } else { 1.0 };
            transform.translate(&(transform_position + (direction * dt as f32 * 100.0 * running)));

            if !self.actions[0] && !self.actions[1] && !self.actions[2] && !self.actions[3] {
                self.animation.offset = 190;
                self.animation.speed = Duration::from_millis(1000 / 6);
            } else if (self.actions[0] || self.actions[1] || self.actions[2] || self.actions[3])
                && !self.actions[6]
            {
                self.animation.offset = 220;
                self.animation.speed = Duration::from_millis(1000 / 10);
            } else {
                self.animation.offset = 200;
                self.animation.speed = Duration::from_millis(1000 / 15);
            }

            self.time_since_last_frame += delta_time;
            if self.time_since_last_frame >= self.animation.speed {
                self.animation.index = (self.animation.index + 1) % self.animation.max;
                // self.uniforms_texture.texture_index = self.animation.index;
                transform.index = self.animation.index + self.animation.offset;

                self.time_since_last_frame -= self.animation.speed;
            }
        }

        // self.queue.write_buffer(
        //     &self.uniform_buffer,
        //     0,
        //     bytemuck::cast_slice(&[self.uniforms_texture]),
        // );

        self.camera.0 = nalgebra_glm::ortho_lh(
            self.transform.as_ref().borrow().position.x - self.size.width as f32 * self.zoom * 0.5,
            self.transform.as_ref().borrow().position.x + self.size.width as f32 * self.zoom * 0.5,
            self.transform.as_ref().borrow().position.y - self.size.height as f32 * self.zoom * 0.5,
            self.transform.as_ref().borrow().position.y + self.size.height as f32 * self.zoom * 0.5,
            0.025,
            1000.0,
        );
        self.camera_uniform.update(self.camera.0, self.camera.1);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        self.renders[3].update_transform(
            bytemuck::cast_slice(&[self.transform.as_ref().borrow().to_raw()]),
            &mut self.queue,
        );

        let instance_data = self
            .instances
            .iter()
            .map(|x| x.as_ref().borrow().to_raw())
            .collect::<Vec<_>>();
        self.renders[4].update_transform(bytemuck::cast_slice(&instance_data), &mut self.queue);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        // load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.renders[0].draw(&mut _render_pass);
            self.renders[1].draw(&mut _render_pass);
            self.renders[2].draw(&mut _render_pass);
            self.renders[3].draw(&mut _render_pass);
            self.renders[4].draw(&mut _render_pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        output.present();

        Ok(())
    }
}
