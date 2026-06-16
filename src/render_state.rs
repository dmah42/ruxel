use std::time::Duration;

use crate::{
    camera,
    camera::{Camera, Projection},
    scene::Scene,
    texture::Texture,
    ui::Ui,
    vertex::{SimpleVertex, Vertex},
};
use rand::Rng;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

const REACH_DISTANCE: f32 = 6.0;

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
    wireframe_pipeline: wgpu::RenderPipeline,
    selected_block: Option<glam::IVec3>,

    ui: Ui,
    scene: Scene,

    depth_texture: Texture,

    camera: Camera,
    projection: Projection,
    camera_uniform: camera::Uniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    light_bind_group: wgpu::BindGroup,
    wireframe_bind_group: wgpu::BindGroup,
}

impl RenderState<'static> {
    pub async fn new(seed: u32, window: Arc<Window>) -> Self {
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
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let depth_texture = Texture::new_depth_texture(&device, &config, "depth texture");

        let ui = Ui::new(&device, &config);
        let scene = Scene::new(seed, &device);

        let mut rng = rand::thread_rng();
        let mut playerx = rng.gen_range(2000.0..4000.0);
        let mut playerz = rng.gen_range(2000.0..4000.0);

        while scene
            .chunks()
            .height_at(&glam::Vec3::new(playerx, 0.0, playerz))
            <= 32.0
        {
            playerx = rng.gen_range(2000.0..4000.0);
            playerz = rng.gen_range(2000.0..4000.0);
        }

        let spawn_height = scene
            .chunks()
            .height_at(&glam::Vec3::new(playerx, 0.0, playerz));
        let camera = Camera::new(
            glam::Vec3::new(playerx, spawn_height + 5.0, playerz),
            0.0,
            0.0,
        );

        let projection = Projection::new(
            config.width as f32 / config.height as f32,
            75.0_f32.to_radians(),
            0.1,
            1000.0,
        );

        let mut camera_uniform = camera::Uniform::new();
        camera_uniform.update_view_proj(&camera, &projection);

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
        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: scene.sky().buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: scene.lights_buffer().as_entire_binding(),
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
                resource: scene.wireframe_uniform_buffer().as_entire_binding(),
            }],
        });

        let render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render pipeline layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout],
                push_constant_ranges: &[],
            });

            PipelineConfig::opaque(&layout, Vertex::desc(), wgpu::include_wgsl!("shader.wgsl"))
                .build(&device, &config, Some(Texture::DEPTH_FORMAT))
        };

        let transparent_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("transparent pipeline layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout],
                push_constant_ranges: &[],
            });

            PipelineConfig::transparent(&layout, Vertex::desc(), wgpu::include_wgsl!("shader.wgsl"))
                .build(&device, &config, Some(Texture::DEPTH_FORMAT))
        };

        let sun_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("sun pipeline layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout],
                push_constant_ranges: &[],
            });

            PipelineConfig::opaque(&layout, SimpleVertex::desc(), wgpu::include_wgsl!("sun.wgsl"))
                .build(&device, &config, Some(Texture::DEPTH_FORMAT))
        };

        let moon_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("moon pipeline layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout],
                push_constant_ranges: &[],
            });

            PipelineConfig::opaque(&layout, SimpleVertex::desc(), wgpu::include_wgsl!("moon.wgsl"))
                .with_blend_state(wgpu::BlendState::ALPHA_BLENDING)
                .build(&device, &config, Some(Texture::DEPTH_FORMAT))
        };

        let wireframe_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("wireframe pipeline layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &wireframe_bind_group_layout],
                push_constant_ranges: &[],
            });

            PipelineConfig::opaque(&layout, SimpleVertex::desc(), wgpu::include_wgsl!("wireframe.wgsl"))
                .with_topology(wgpu::PrimitiveTopology::LineList)
                .build(&device, &config, Some(Texture::DEPTH_FORMAT))
        };

        Self {
            size,
            surface,
            device,
            queue,
            config,
            render_pipeline,
            transparent_pipeline,
            sun_render_pipeline,
            moon_render_pipeline,
            wireframe_pipeline,
            selected_block: None,
            ui,
            scene,
            depth_texture,
            camera,
            projection,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            light_bind_group,
            wireframe_bind_group,
        }
    }

    pub fn camera(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn update_physics(&mut self, dt: Duration) {
        self.camera.update_physics(self.scene.chunks(), dt);
    }

    pub fn interact(&mut self, place: bool, block_type: crate::block::Type) {
        if let Some((hit_pos, normal)) = self.camera.raycast(self.scene.chunks(), REACH_DISTANCE) {
            if place {
                let p = hit_pos + normal;
                self.scene.chunks().set_block(p.x, p.y, p.z, block_type);
            } else {
                self.scene.chunks().set_block(
                    hit_pos.x,
                    hit_pos.y,
                    hit_pos.z,
                    crate::block::Type::Inactive,
                );
            }
        }
    }

    pub fn update(&mut self, dt: Duration, selected_block_type: crate::block::Type) {
        self.ui.update(
            &self.camera.position(),
            self.scene.chunks().block_position(),
            self.scene.chunks().chunk_position(),
            selected_block_type,
            dt,
        );
        self.scene.update(dt, &self.camera.position(), &self.device);

        self.selected_block = self.camera.raycast(self.scene.chunks(), REACH_DISTANCE).map(|(pos, _)| pos);
        if let Some(pos) = self.selected_block {
            let offset = [pos.x as f32, pos.y as f32, pos.z as f32, 0.0f32];
            self.queue.write_buffer(self.scene.wireframe_uniform_buffer(), 0, bytemuck::cast_slice(&[offset]));
        }

        self.camera_uniform
            .update_view_proj(&self.camera, &self.projection);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        self.queue.write_buffer(
            self.scene.lights_buffer(),
            0,
            bytemuck::cast_slice(&[self.scene.lights().to_raw()]),
        );

        self.queue.write_buffer(
            self.scene.sky().buffer(),
            0,
            bytemuck::cast_slice(&[self.scene.sky().to_raw(self.config.format.is_srgb())]),
        );
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture =
                Texture::new_depth_texture(&self.device, &self.config, "depth buffer");
            self.projection.resize(new_size.width, new_size.height);
            self.ui.resize(new_size, &self.queue);
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.scene.sky().color()),
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
                render_pass.set_vertex_buffer(0, self.scene.vertex_buffer().slice(..));
                render_pass.set_index_buffer(
                    self.scene.index_buffer().slice(..),
                    wgpu::IndexFormat::Uint16,
                );
                render_pass.draw_indexed(0..self.scene.num_indices(), 0, 0..1);
            }
            // draw moon
            {
                render_pass.set_pipeline(&self.moon_render_pipeline);
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                render_pass.set_bind_group(1, &self.light_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.scene.vertex_buffer().slice(..));
                render_pass.set_index_buffer(
                    self.scene.index_buffer().slice(..),
                    wgpu::IndexFormat::Uint16,
                );
                render_pass.draw_indexed(0..self.scene.num_indices(), 0, 0..1);
            }

            // draw landscape
            {
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                render_pass.set_bind_group(1, &self.light_bind_group, &[]);

                for chunk_col in self.scene.chunk_buffers().values() {
                    for chunk in chunk_col.iter().flatten() {
                        if let Some((index_buf, num_indices)) = &chunk.opaque_index_buffer {
                            render_pass.set_vertex_buffer(0, chunk.vertex_buffer.slice(..));
                            render_pass
                                .set_index_buffer(index_buf.slice(..), wgpu::IndexFormat::Uint32);
                            render_pass.draw_indexed(0..*num_indices, 0, 0..1);
                        }
                    }
                }
            }

            // draw transparent blocks (water)
            {
                render_pass.set_pipeline(&self.transparent_pipeline);
                render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                render_pass.set_bind_group(1, &self.light_bind_group, &[]);

                for chunk_col in self.scene.chunk_buffers().values() {
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
                render_pass.set_vertex_buffer(0, self.scene.vertex_buffer().slice(..));
                render_pass.set_index_buffer(self.scene.wireframe_index_buffer().slice(..), wgpu::IndexFormat::Uint16);
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
            self.ui.render(&self.device, &self.queue, &mut ui_pass);
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
