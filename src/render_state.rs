use std::time::Duration;

use crate::{
    camera::{Camera, Uniform},
    scene::Scene,
    sky::Sky,
    texture::Texture,
    ui::Ui,
    vertex::{SimpleVertex, Vertex},
};
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu_text::glyph_brush::{HorizontalAlign, VerticalAlign};
use wgpu_text::{
    glyph_brush::{ab_glyph::FontArc, Layout, Section, Text},
    BrushBuilder, TextBrush,
};
use winit::window::Window;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct MainShadowUniform {
    sun_view_proj: [f32; 16],
    moon_view_proj: [f32; 16],
}

impl MainShadowUniform {
    pub fn new() -> Self {
        Self {
            sun_view_proj: *glam::Mat4::IDENTITY.as_ref(),
            moon_view_proj: *glam::Mat4::IDENTITY.as_ref(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct ShadowPassUniform {
    view_proj: [f32; 16],
}

impl ShadowPassUniform {
    pub fn new() -> Self {
        Self {
            view_proj: *glam::Mat4::IDENTITY.as_ref(),
        }
    }
}

struct ChunkBuffers {
    vertex_buffer: wgpu::Buffer,
    opaque_index_buffer: Option<(wgpu::Buffer, u32)>,
    transparent_index_buffer: Option<(wgpu::Buffer, u32)>,
}

struct EntityBuffers {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}

pub struct RenderState<'window> {
    pub size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    transparent_pipeline: wgpu::RenderPipeline,
    sun_render_pipeline: wgpu::RenderPipeline,
    moon_render_pipeline: wgpu::RenderPipeline,
    sky_render_pipeline: wgpu::RenderPipeline,
    wireframe_pipeline: wgpu::RenderPipeline,
    overlay_pipeline: wgpu::RenderPipeline,
    selected_block: Option<glam::IVec3>,

    ui_brush: TextBrush,

    depth_texture: Texture,

    camera_uniform: Uniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    light_bind_group: wgpu::BindGroup,
    wireframe_bind_group: wgpu::BindGroup,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    wireframe_index_buffer: wgpu::Buffer,
    wireframe_uniform_buffer: wgpu::Buffer,

    sun_shadow_texture: Texture,
    moon_shadow_texture: Texture,
    shadow_pipeline: wgpu::RenderPipeline,

    main_shadow_bind_group: wgpu::BindGroup,
    main_shadow_uniform_buffer: wgpu::Buffer,

    sun_shadow_pass_bind_group: wgpu::BindGroup,
    moon_shadow_pass_bind_group: wgpu::BindGroup,
    sun_shadow_pass_uniform_buffer: wgpu::Buffer,
    moon_shadow_pass_uniform_buffer: wgpu::Buffer,

    entity_buffers: std::collections::HashMap<glam::UVec2, EntityBuffers>,

    chunk_buffers: std::collections::HashMap<glam::UVec2, Vec<Option<ChunkBuffers>>>,
    chunk_versions: std::collections::HashMap<glam::UVec2, Vec<u32>>,

    lights_buffer: wgpu::Buffer,

    sky: Sky,
}

impl RenderState<'static> {
    pub async fn new(_config: crate::config::Config, window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance
            .create_surface(window)
            .expect("failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("failed to create adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None,
            )
            .await
            .expect("failed to create device and queue");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let depth_texture = Texture::new_depth_texture(&device, &surface_config, "depth texture");

        let font_data = include_bytes!("../fonts/Stacked pixel.ttf").to_vec();
        let font = FontArc::try_from_vec(font_data).expect("unable to load font");
        let ui_brush = BrushBuilder::using_font(font).build(
            &device,
            surface_config.width,
            surface_config.height,
            surface_config.format,
        );

        let sun_shadow_texture =
            Texture::new_depth_texture_with_size(&device, 2048, 2048, "sun shadow depth texture");
        let moon_shadow_texture =
            Texture::new_depth_texture_with_size(&device, 2048, 2048, "moon shadow depth texture");
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let main_shadow_uniform = MainShadowUniform::new();
        let main_shadow_uniform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("main shadow uniform buffer"),
                contents: bytemuck::cast_slice(&[main_shadow_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let shadow_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("shadow bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        // sun shadow map
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        // moon shadow map
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        // sampler
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        // uniform
                        binding: 3,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let main_shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("main shadow bind group"),
            layout: &shadow_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&sun_shadow_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&moon_shadow_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: main_shadow_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let shadow_pass_uniform = ShadowPassUniform::new();
        let sun_shadow_pass_uniform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sun shadow pass uniform buffer"),
                contents: bytemuck::cast_slice(&[shadow_pass_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let moon_shadow_pass_uniform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("moon shadow pass uniform buffer"),
                contents: bytemuck::cast_slice(&[shadow_pass_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let shadow_pass_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("shadow pass bind group layout"),
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

        let sun_shadow_pass_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sun shadow pass bind group"),
            layout: &shadow_pass_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sun_shadow_pass_uniform_buffer.as_entire_binding(),
            }],
        });
        let moon_shadow_pass_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("moon shadow pass bind group"),
            layout: &shadow_pass_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: moon_shadow_pass_uniform_buffer.as_entire_binding(),
            }],
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex buffer"),
            contents: bytemuck::cast_slice(crate::vertex::CUBE_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index buffer"),
            contents: bytemuck::cast_slice(crate::vertex::CUBE_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = crate::vertex::CUBE_INDICES.len() as u32;

        let wireframe_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("wireframe index buffer"),
            contents: bytemuck::cast_slice(crate::vertex::WIREFRAME_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let wireframe_uniform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("wireframe uniform buffer"),
                contents: bytemuck::cast_slice(&[[0.0f32; 4]]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let camera_uniform = Uniform::new();

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera bind group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let sky = Sky::new(&device);

        // Dummy lights initialization. The actual lights will be written during update.
        let dummy_lights = crate::scene::Lights::empty();
        let lights_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lights buffer"),
            contents: bytemuck::cast_slice(&[dummy_lights.to_raw()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: sky.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: lights_buffer.as_entire_binding(),
                },
            ],
            label: Some("light bind group"),
        });

        let wireframe_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("wireframe bind group layout"),
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

        let wireframe_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wireframe bind group"),
            layout: &wireframe_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wireframe_uniform_buffer.as_entire_binding(),
            }],
        });

        let render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render pipeline layout"),
                bind_group_layouts: &[
                    &camera_bind_group_layout,
                    &light_bind_group_layout,
                    &shadow_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

            PipelineConfig::opaque(&layout, Vertex::desc(), wgpu::include_wgsl!("shader.wgsl"))
                .build(&device, &surface_config, Some(Texture::DEPTH_FORMAT))
        };

        let transparent_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("transparent pipeline layout"),
                bind_group_layouts: &[
                    &camera_bind_group_layout,
                    &light_bind_group_layout,
                    &shadow_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

            PipelineConfig::transparent(&layout, Vertex::desc(), wgpu::include_wgsl!("shader.wgsl"))
                .build(&device, &surface_config, Some(Texture::DEPTH_FORMAT))
        };

        let sun_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("sun pipeline layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout],
                push_constant_ranges: &[],
            });

            PipelineConfig::opaque(
                &layout,
                SimpleVertex::desc(),
                wgpu::include_wgsl!("sun.wgsl"),
            )
            .build(&device, &surface_config, Some(Texture::DEPTH_FORMAT))
        };

        let moon_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("moon pipeline layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout],
                push_constant_ranges: &[],
            });

            PipelineConfig::opaque(
                &layout,
                SimpleVertex::desc(),
                wgpu::include_wgsl!("moon.wgsl"),
            )
            .with_blend_state(wgpu::BlendState::ALPHA_BLENDING)
            .build(&device, &surface_config, Some(Texture::DEPTH_FORMAT))
        };

        let wireframe_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("wireframe pipeline layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &wireframe_bind_group_layout],
                push_constant_ranges: &[],
            });

            PipelineConfig::opaque(
                &layout,
                SimpleVertex::desc(),
                wgpu::include_wgsl!("wireframe.wgsl"),
            )
            .with_topology(wgpu::PrimitiveTopology::LineList)
            .build(&device, &surface_config, Some(Texture::DEPTH_FORMAT))
        };

        let shadow_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("shadow pipeline layout"),
                bind_group_layouts: &[&shadow_pass_bind_group_layout],
                push_constant_ranges: &[],
            });

            let shader = device.create_shader_module(wgpu::include_wgsl!("shadow.wgsl"));
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("shadow render pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[Vertex::desc()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Front), // TODO: NoNE?
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: Texture::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState {
                        constant: 2,
                        slope_scale: 2.0,
                        clamp: 0.0,
                    },
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: None,
                multiview: None,
            })
        };

        let sky_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("sky pipeline layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout],
                push_constant_ranges: &[],
            });

            let shader = device.create_shader_module(wgpu::include_wgsl!("sky.wgsl"));
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("sky render pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: Texture::DEPTH_FORMAT,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                multiview: None,
            })
        };

        let overlay_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("overlay pipeline layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

            let shader = device.create_shader_module(wgpu::include_wgsl!("overlay.wgsl"));
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("overlay render pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_config.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                multiview: None,
            })
        };

        Self {
            size,
            surface,
            device,
            queue,
            config: surface_config,
            render_pipeline,
            transparent_pipeline,
            sun_render_pipeline,
            moon_render_pipeline,
            sky_render_pipeline,
            wireframe_pipeline,
            overlay_pipeline,
            chunk_buffers: std::collections::HashMap::new(),
            chunk_versions: std::collections::HashMap::new(),

            lights_buffer,
            sky,

            selected_block: None,
            sun_shadow_texture,
            moon_shadow_texture,
            shadow_pipeline,
            main_shadow_bind_group,
            main_shadow_uniform_buffer,
            sun_shadow_pass_bind_group,
            moon_shadow_pass_bind_group,
            sun_shadow_pass_uniform_buffer,
            moon_shadow_pass_uniform_buffer,
            ui_brush,
            depth_texture,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            light_bind_group,
            wireframe_bind_group,
            vertex_buffer,
            index_buffer,
            num_indices,
            wireframe_index_buffer,
            wireframe_uniform_buffer,
            entity_buffers: std::collections::HashMap::new(),
        }
    }

    pub fn update(
        &mut self,
        dt: Duration,
        camera: &Camera,
        scene: &Scene,
        selected_block: Option<glam::IVec3>,
    ) {
        self.update_chunk_buffers(scene);

        self.selected_block = selected_block;
        if let Some(pos) = self.selected_block {
            let offset = [pos.x as f32, pos.y as f32, pos.z as f32, 0.0f32];
            self.queue.write_buffer(
                &self.wireframe_uniform_buffer,
                0,
                bytemuck::cast_slice(&[offset]),
            );
        }

        self.camera_uniform.update_view_proj(camera);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        self.queue.write_buffer(
            &self.lights_buffer,
            0,
            bytemuck::cast_slice(&[scene.lights().to_raw()]),
        );

        let sun_pos = scene.sun_position();
        let moon_pos = scene.moon_position();

        let size = (scene.load_radius() as f32 * 16.0) * std::f32::consts::SQRT_2;
        let shadow_proj = glam::Mat4::orthographic_rh(-size, size, -size, size, -200.0, 200.0);
        // Sometimes sun view looks exactly down which breaks look_at_rh, so we add a tiny epsilon to the up vector
        let sun_up = if (sun_pos - camera.position())
            .normalize_or_zero()
            .dot(glam::Vec3::Y)
            .abs()
            > 0.999
        {
            glam::Vec3::X
        } else {
            glam::Vec3::Y
        };
        let sun_view = glam::Mat4::look_at_rh(sun_pos, camera.position(), sun_up);
        // Sometimes moon view looks exactly down which breaks look_at_rh, so we add a tiny epsilon to the up vector
        let up = if (moon_pos - camera.position())
            .normalize_or_zero()
            .dot(glam::Vec3::Y)
            .abs()
            > 0.999
        {
            glam::Vec3::X
        } else {
            glam::Vec3::Y
        };
        let moon_view = glam::Mat4::look_at_rh(moon_pos, camera.position(), up);

        let sun_view_proj = shadow_proj * sun_view;
        let moon_view_proj = shadow_proj * moon_view;

        let main_shadow_uniform = MainShadowUniform {
            sun_view_proj: *sun_view_proj.as_ref(),
            moon_view_proj: *moon_view_proj.as_ref(),
        };
        self.queue.write_buffer(
            &self.main_shadow_uniform_buffer,
            0,
            bytemuck::cast_slice(&[main_shadow_uniform]),
        );

        let sun_shadow_pass_uniform = ShadowPassUniform {
            view_proj: *sun_view_proj.as_ref(),
        };
        self.queue.write_buffer(
            &self.sun_shadow_pass_uniform_buffer,
            0,
            bytemuck::cast_slice(&[sun_shadow_pass_uniform]),
        );

        let moon_shadow_pass_uniform = ShadowPassUniform {
            view_proj: *moon_view_proj.as_ref(),
        };
        self.queue.write_buffer(
            &self.moon_shadow_pass_uniform_buffer,
            0,
            bytemuck::cast_slice(&[moon_shadow_pass_uniform]),
        );

        self.sky.update(dt, &scene.sun_offset());
        self.queue.write_buffer(
            self.sky.buffer(),
            0,
            bytemuck::cast_slice(&[self.sky.to_raw(self.config.format.is_srgb())]),
        );

        let loaded_cells = scene.entity_manager().loaded_cells();

        // Remove buffers for entities that are no longer loaded
        self.entity_buffers
            .retain(|key, _| loaded_cells.contains_key(key));

        // Create buffers for newly loaded entity chunks
        for (key, mesh) in loaded_cells {
            if !self.entity_buffers.contains_key(key) && !mesh.vertices.is_empty() {
                use wgpu::util::DeviceExt;
                let vertex_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("entity chunk vertex buffer"),
                            contents: bytemuck::cast_slice(&mesh.vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });
                let index_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("entity chunk index buffer"),
                            contents: bytemuck::cast_slice(&mesh.indices),
                            usage: wgpu::BufferUsages::INDEX,
                        });
                self.entity_buffers.insert(
                    *key,
                    EntityBuffers {
                        vertex_buffer,
                        index_buffer,
                        num_indices: mesh.indices.len() as u32,
                    },
                );
            }
        }
    }

    fn update_chunk_buffers(&mut self, scene: &Scene) {
        let loaded = scene.chunks().loaded();
        let locked_loaded = loaded.lock().expect("");

        // Remove buffers for chunks that are no longer loaded
        self.chunk_buffers
            .retain(|key, _| locked_loaded.contains_key(key));
        self.chunk_versions
            .retain(|key, _| locked_loaded.contains_key(key));

        let terrain = scene.chunks().terrain().clone();

        let mut dirty_chunks = Vec::new();
        for (key, chunks) in locked_loaded.iter() {
            let versions = self
                .chunk_versions
                .entry(*key)
                .or_insert_with(|| vec![0; chunks.len()]);
            for (i, chunk) in chunks.iter().enumerate() {
                if versions[i] != chunk.version() {
                    dirty_chunks.push((*key, i, chunk.version()));
                }
            }
        }

        let mut new_meshes = Vec::new();
        for (key, i, version) in dirty_chunks {
            let chunk = &locked_loaded.get(&key).unwrap()[i];
            let mesh = crate::mesh::ChunkMesh::build(chunk, &locked_loaded, &terrain);
            new_meshes.push((key, i, version, mesh));
        }

        for (key, i, version, mesh) in new_meshes {
            let col_buffers = self.chunk_buffers.entry(key).or_insert_with(|| {
                (0..locked_loaded.get(&key).unwrap().len())
                    .map(|_| None)
                    .collect()
            });
            let col_versions = self
                .chunk_versions
                .entry(key)
                .or_insert_with(|| vec![0; locked_loaded.get(&key).unwrap().len()]);

            let opaque_indices = mesh.opaque_indices();
            let transparent_indices = mesh.transparent_indices();

            if opaque_indices.is_empty() && transparent_indices.is_empty() {
                col_buffers[i] = None;
            } else {
                let vertex_buffer =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("chunk vertex buffer"),
                            contents: bytemuck::cast_slice(mesh.vertices()),
                            usage: wgpu::BufferUsages::VERTEX,
                        });

                let opaque_index_buffer = if !opaque_indices.is_empty() {
                    let buf = self
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("chunk opaque index buffer"),
                            contents: bytemuck::cast_slice(opaque_indices),
                            usage: wgpu::BufferUsages::INDEX,
                        });
                    Some((buf, opaque_indices.len() as u32))
                } else {
                    None
                };

                let transparent_index_buffer = if !transparent_indices.is_empty() {
                    let buf = self
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("chunk transparent index buffer"),
                            contents: bytemuck::cast_slice(transparent_indices),
                            usage: wgpu::BufferUsages::INDEX,
                        });
                    Some((buf, transparent_indices.len() as u32))
                } else {
                    None
                };

                col_buffers[i] = Some(ChunkBuffers {
                    vertex_buffer,
                    opaque_index_buffer,
                    transparent_index_buffer,
                });
            }
            col_versions[i] = version;
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture =
                Texture::new_depth_texture(&self.device, &self.config, "depth buffer");
            self.ui_brush
                .resize_view(new_size.width as f32, new_size.height as f32, &self.queue);
        }
    }

    pub fn render(&mut self, ui: &Ui) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("command encoder"),
            });

        {
            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sun shadow pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.sun_shadow_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            shadow_pass.set_pipeline(&self.shadow_pipeline);
            shadow_pass.set_bind_group(0, &self.sun_shadow_pass_bind_group, &[]);

            // draw entities in shadow pass
            for buffers in self.entity_buffers.values() {
                shadow_pass.set_vertex_buffer(0, buffers.vertex_buffer.slice(..));
                shadow_pass
                    .set_index_buffer(buffers.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                shadow_pass.draw_indexed(0..buffers.num_indices, 0, 0..1);
            }
        }

        {
            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("moon shadow pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.moon_shadow_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            shadow_pass.set_pipeline(&self.shadow_pipeline);
            shadow_pass.set_bind_group(0, &self.moon_shadow_pass_bind_group, &[]);

            // draw entities in shadow pass
            for buffers in self.entity_buffers.values() {
                shadow_pass.set_vertex_buffer(0, buffers.vertex_buffer.slice(..));
                shadow_pass
                    .set_index_buffer(buffers.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                shadow_pass.draw_indexed(0..buffers.num_indices, 0, 0..1);
            }
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.sky.color()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // draw sun
            {
                render_pass.set_pipeline(&self.sun_render_pipeline);
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                render_pass.set_bind_group(1, &self.light_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
            }
            // draw moon
            {
                render_pass.set_pipeline(&self.moon_render_pipeline);
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                render_pass.set_bind_group(1, &self.light_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
            }

            // draw landscape
            {
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                render_pass.set_bind_group(1, &self.light_bind_group, &[]);
                render_pass.set_bind_group(2, &self.main_shadow_bind_group, &[]);

                for chunk_col in self.chunk_buffers.values() {
                    for chunk in chunk_col.iter().flatten() {
                        if let Some((index_buf, num_indices)) = &chunk.opaque_index_buffer {
                            render_pass.set_vertex_buffer(0, chunk.vertex_buffer.slice(..));
                            render_pass
                                .set_index_buffer(index_buf.slice(..), wgpu::IndexFormat::Uint32);
                            render_pass.draw_indexed(0..*num_indices, 0, 0..1);
                        }
                    }
                }

                for entity_buf in self.entity_buffers.values() {
                    render_pass.set_vertex_buffer(0, entity_buf.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(
                        entity_buf.index_buffer.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    render_pass.draw_indexed(0..entity_buf.num_indices, 0, 0..1);
                }
            }

            // draw sky
            {
                render_pass.set_pipeline(&self.sky_render_pipeline);
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                render_pass.set_bind_group(1, &self.light_bind_group, &[]);
                render_pass.draw(0..3, 0..1);
            }

            // draw transparent blocks (water)
            {
                render_pass.set_pipeline(&self.transparent_pipeline);
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                render_pass.set_bind_group(1, &self.light_bind_group, &[]);
                render_pass.set_bind_group(2, &self.main_shadow_bind_group, &[]);

                for chunk_col in self.chunk_buffers.values() {
                    for chunk in chunk_col.iter().flatten() {
                        if let Some((index_buf, num_indices)) = &chunk.transparent_index_buffer {
                            render_pass.set_vertex_buffer(0, chunk.vertex_buffer.slice(..));
                            render_pass
                                .set_index_buffer(index_buf.slice(..), wgpu::IndexFormat::Uint32);
                            render_pass.draw_indexed(0..*num_indices, 0, 0..1);
                        }
                    }
                }
            }

            // draw wireframe
            if self.selected_block.is_some() {
                render_pass.set_pipeline(&self.wireframe_pipeline);
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                render_pass.set_bind_group(1, &self.wireframe_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_index_buffer(
                    self.wireframe_index_buffer.slice(..),
                    wgpu::IndexFormat::Uint16,
                );
                render_pass.draw_indexed(0..24, 0, 0..1);
            }
        }

        {
            let mut ui_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            if ui.is_console_open {
                ui_pass.set_pipeline(&self.overlay_pipeline);
                ui_pass.draw(0..6, 0..1);
            }

            let center = (self.size.width as f32 / 2.0, self.size.height as f32 / 2.0);

            let mut text_sections = vec![Section::default()
                .add_text(
                    Text::new(&ui.target)
                        .with_scale(48.0)
                        .with_color([0.0, 0.0, 0.0, 0.7]),
                )
                .with_screen_position(center)];

            if ui.is_console_open {
                let scale = 28.0;
                text_sections.extend(vec![
                    Section::default()
                        .add_text(
                            Text::new(&ui.player_position)
                                .with_scale(scale)
                                .with_color([0.2, 0.8, 1.0, 1.0]),
                        )
                        .with_screen_position((20.0, 20.0)),
                    Section::default()
                        .add_text(
                            Text::new(&ui.block_position)
                                .with_scale(scale)
                                .with_color([0.3, 0.9, 0.9, 1.0]),
                        )
                        .with_screen_position((20.0, 55.0)),
                    Section::default()
                        .add_text(
                            Text::new(&ui.chunk_position)
                                .with_scale(scale)
                                .with_color([0.9, 0.7, 0.3, 1.0]),
                        )
                        .with_screen_position((20.0, 90.0)),
                    Section::default()
                        .add_text(
                            Text::new(&ui.fps_str)
                                .with_scale(scale)
                                .with_color([0.2, 1.0, 0.2, 1.0]),
                        )
                        .with_screen_position((self.size.width as f32 / 2.0, 20.0))
                        .with_layout(Layout::default().h_align(HorizontalAlign::Center)),
                    Section::default()
                        .add_text(
                            Text::new(&ui.selected_block)
                                .with_scale(scale)
                                .with_color([0.9, 0.5, 0.9, 1.0]),
                        )
                        .with_screen_position((self.size.width as f32 - 20.0, 20.0))
                        .with_layout(Layout::default().h_align(HorizontalAlign::Right)),
                    Section::default()
                        .add_text(
                            Text::new(&ui.biome)
                                .with_scale(scale)
                                .with_color([1.0, 0.9, 0.4, 1.0]),
                        )
                        .with_screen_position((self.size.width as f32 - 20.0, 55.0))
                        .with_layout(Layout::default().h_align(HorizontalAlign::Right)),
                    {
                        let console_bottom = self.size.height as f32 * 0.42;
                        let layout = Layout::default().v_align(VerticalAlign::Bottom);

                        Section::default()
                            .add_text(
                                Text::new(&ui.console_text)
                                    .with_scale(scale)
                                    .with_color([1.0, 1.0, 1.0, 1.0]),
                            )
                            .with_screen_position((20.0, console_bottom - 20.0))
                            .with_layout(layout)
                            .with_bounds((self.size.width as f32 - 40.0, console_bottom - 150.0))
                    },
                ]);
            }

            self.ui_brush
                .queue(&self.device, &self.queue, text_sections)
                .expect("failed to process UI queue");
            self.ui_brush.draw(&mut ui_pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

struct PipelineConfig<'a> {
    layout: &'a wgpu::PipelineLayout,
    vertex_buffer_layout: wgpu::VertexBufferLayout<'a>,
    shader: wgpu::ShaderModuleDescriptor<'a>,
    depth_write_enabled: bool,
    blend_state: wgpu::BlendState,
    cull_mode: Option<wgpu::Face>,
    topology: wgpu::PrimitiveTopology,
}

impl<'a> PipelineConfig<'a> {
    fn opaque(
        layout: &'a wgpu::PipelineLayout,
        vertex_buffer_layout: wgpu::VertexBufferLayout<'a>,
        shader: wgpu::ShaderModuleDescriptor<'a>,
    ) -> Self {
        Self {
            layout,
            vertex_buffer_layout,
            shader,
            depth_write_enabled: true,
            blend_state: wgpu::BlendState::REPLACE,
            cull_mode: Some(wgpu::Face::Back),
            topology: wgpu::PrimitiveTopology::TriangleList,
        }
    }

    fn transparent(
        layout: &'a wgpu::PipelineLayout,
        vertex_buffer_layout: wgpu::VertexBufferLayout<'a>,
        shader: wgpu::ShaderModuleDescriptor<'a>,
    ) -> Self {
        Self {
            layout,
            vertex_buffer_layout,
            shader,
            depth_write_enabled: false,
            blend_state: wgpu::BlendState::ALPHA_BLENDING,
            cull_mode: None,
            topology: wgpu::PrimitiveTopology::TriangleList,
        }
    }

    fn with_blend_state(mut self, blend_state: wgpu::BlendState) -> Self {
        self.blend_state = blend_state;
        self
    }

    fn with_topology(mut self, topology: wgpu::PrimitiveTopology) -> Self {
        self.topology = topology;
        self
    }

    fn build(
        self,
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(self.shader);

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render pipeline"),
            layout: Some(self.layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[self.vertex_buffer_layout],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: self.topology,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: self.cull_mode,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
                format,
                depth_write_enabled: self.depth_write_enabled,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(self.blend_state),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview: None,
        })
    }
}
