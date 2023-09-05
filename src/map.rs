use wgpu::util::DeviceExt;

#[cfg(target_arch = "wasm32")]
use web_sys::{Request, RequestInit, RequestMode, Response};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::*;

#[cfg(target_arch = "wasm32")]
use web_sys::Blob;

#[derive(Debug)]
pub struct TileSet {
    pub image: Vec<u8>,
    pub columns: u32,
    pub tile_count: u32,
    pub image_size: (u32, u32),
}

#[derive(Debug)]
pub struct Object {
    pub name: String,
    pub id: u32,
    pub position: (u32, u32),
    pub size: (u32, u32),
}

#[derive(Debug)]
pub struct TileLayer {
    pub data: Vec<u32>,
    pub id: u32,
    pub name: String,
    pub size: (u32, u32),
}

#[derive(Debug)]
pub enum Layers {
    ObjectGroup {
        id: u32,
        name: String,
        visible: bool,
        objects: Vec<Object>,
    },
    TileLayer {
        id: u32,
        name: String,
        visible: bool,
        data: Vec<i32>,
    },
}

#[derive(Debug)]
pub struct Map {
    pub size: (u32, u32),
    pub tile_size: (u32, u32),
    pub layers: Vec<Layers>,
    pub tileset: TileSet,
}

pub async fn load_map(path_data: &str) -> Map {
    let json_file = {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                let mut opts = RequestInit::new();
                opts.method("GET");
                opts.mode(RequestMode::Cors);

                let url = format!("{}", path_data);
                let request = Request::new_with_str_and_init(&url, &opts).unwrap();
                let window = web_sys::window().unwrap();
                let resp_value = JsFuture::from(window.fetch_with_request(&request))
                    .await
                    .unwrap();
                let resp: Response = resp_value.dyn_into().unwrap();
                let json = JsFuture::from(resp.json().unwrap()).await.unwrap();
                serde_wasm_bindgen::from_value::<serde_json::Value>(json).unwrap()
            } else {
                let data  = std::fs::read_to_string(path_data).unwrap();
                serde_json::from_str::<serde_json::Value>(data.as_str()).unwrap()
            }
        }
    };

    let filename = {
        let mut f = "./resources/".to_owned();
        f.push_str(json_file["tilesets"][0]["image"].as_str().unwrap());
        f
    };

    let spritesheet = {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                let mut opts = RequestInit::new();
                opts.method("GET");
                opts.mode(RequestMode::Cors);

                let url = format!("{}", filename.as_str());
                let request = Request::new_with_str_and_init(&url, &opts).unwrap();
                let window = web_sys::window().unwrap();
                let resp_value = JsFuture::from(window.fetch_with_request(&request))
                    .await
                    .unwrap();
                let resp: Response = resp_value.dyn_into().unwrap();
                let blob_response = JsFuture::from(resp.blob().unwrap()).await.unwrap();

                let blob: Blob = blob_response.into();

                let array_buffer_promise: JsFuture = blob.array_buffer().into();
                let array_buffer: JsValue = array_buffer_promise.await.unwrap();
                js_sys::Uint8Array::new(&array_buffer).to_vec()
            } else {
                std::fs::read(filename).unwrap()
            }
        }
    };

    let map = Map {
        size: (
            json_file["width"].as_u64().unwrap() as u32,
            json_file["height"].as_u64().unwrap() as u32,
        ),
        tile_size: (
            json_file["tilewidth"].as_u64().unwrap() as u32,
            json_file["tileheight"].as_u64().unwrap() as u32,
        ),
        layers: {
            let mut layers: Vec<Layers> = vec![];
            for value in json_file["layers"].as_array().unwrap() {
                match value["type"].as_str().unwrap() {
                    "tilelayer" => {
                        layers.push(Layers::TileLayer {
                            id: value["id"].as_u64().unwrap() as u32,
                            name: value["name"].as_str().unwrap().to_string(),
                            visible: value["visible"].as_bool().unwrap(),
                            data: value["data"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|x| (x.as_i64().unwrap() as i32) - 1)
                                .collect::<Vec<i32>>(),
                        });
                    }
                    "objectgroup" => {
                        // println!("OBJECT GROUP")
                    }
                    _ => {}
                }
            }
            layers
        },
        tileset: TileSet {
            image: spritesheet,
            columns: json_file["tilesets"][0]["columns"].as_u64().unwrap() as u32,
            tile_count: json_file["tilesets"][0]["tilecount"].as_u64().unwrap() as u32,
            image_size: (
                json_file["tilesets"][0]["tilewidth"].as_u64().unwrap() as u32,
                json_file["tilesets"][0]["tileheight"].as_u64().unwrap() as u32,
            ),
        },
    };

    map
}

pub fn generate_render(
    id: usize,
    map: &Map,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    camera_bind_group: std::rc::Rc<wgpu::BindGroup>,
    surface_format: &wgpu::TextureFormat,
) -> super::render::Render {
    let diffuse_bytes = &map.tileset.image;
    let diffuse_texture = super::texture::Texture::from_bytes(
        &device,
        &queue,
        &diffuse_bytes,
        format!("spritesheet{}.png", id).as_str(),
    )
    .unwrap();
    let texture_bind_group_layout: wgpu::BindGroupLayout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
    if let Layers::TileLayer { data, .. } = &map.layers[id] {
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
                let mut t = super::transform::Transform::new();
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

    let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Instance Buffer"),
        contents: bytemuck::cast_slice(&instance_data),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });

    let (vertex_points, vertex_indices) =
        super::vertex::get_rect(nalgebra_glm::vec3(16.0, 16.0, 0.0));
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

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
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
                        array_stride: std::mem::size_of::<super::vertex::Vertex>()
                            as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            wgpu::VertexAttribute {
                                offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                        ],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<super::transform::TransformRaw>()
                            as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 5,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                                shader_location: 6,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                                shader_location: 7,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: std::mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                                shader_location: 8,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            wgpu::VertexAttribute {
                                offset: std::mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                                shader_location: 9,
                                format: wgpu::VertexFormat::Sint32,
                            },
                            wgpu::VertexAttribute {
                                offset: std::mem::size_of::<[f32; 17]>() as wgpu::BufferAddress,
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
                    format: *surface_format,
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

    super::render::Render {
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
}
