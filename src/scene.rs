use std::time::Duration;

use crate::{
    block::Block,
    chunks::Chunks,
    instance::{Instance, RawInstance},
    light::{Light, RawLight},
    sky::Sky,
};
use bytemuck::{Pod, Zeroable};
use glam::{Quat, Vec3};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawLights {
    lights: [RawLight; 2],
}

pub struct Lights {
    lights: Vec<Light>,
}

impl Lights {
    pub fn to_raw(&self) -> RawLights {
        let mut raw_lights = RawLights {
            lights: [RawLight::new(); 2],
        };
        for (idx, light) in self.lights.iter().enumerate() {
            raw_lights.lights[idx] = light.to_raw();
        }
        raw_lights
    }
}

pub struct Scene {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    chunks: Chunks,
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,

    sun_offset: Vec3,
    moon_offset: Vec3,

    lights: Lights,
    lights_buffer: wgpu::Buffer,

    sky: Sky,
}

impl Scene {
    pub fn new(seed: u32, device: &wgpu::Device) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex buffer"),
            contents: bytemuck::cast_slice(Block::VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index buffer"),
            contents: bytemuck::cast_slice(Block::INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = Block::INDICES.len() as u32;

        let chunks = Chunks::new(seed);

        // TODO: position sun relative to player always.
        let lights = Lights {
            lights: vec![
                // sun
                Light::new(
                    Vec3::new(0.0, 200.0, 50.0),
                    wgpu::Color {
                        r: 0.99,
                        g: 0.85,
                        b: 0.21,
                        a: 1.0,
                    },
                ),
                // moon
                Light::new(
                    Vec3::new(0.0, -80.0, 40.0),
                    wgpu::Color {
                        r: 0.76,
                        g: 0.77,
                        b: 0.80,
                        a: 0.5,
                    },
                ),
            ],
        };
        let lights_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lights buffer"),
            contents: bytemuck::cast_slice(&[lights.to_raw()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let instance_data: Vec<RawInstance> = vec![];
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("instance buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            num_indices,

            instances: vec![],
            instance_buffer,

            sun_offset: Vec3::new(64.0, 64.0, 0.0),
            moon_offset: Vec3::new(-64.0, -64.0, 32.0),

            lights,
            lights_buffer,
            sky: Sky::new(device),

            chunks,
        }
    }

    pub fn chunks(&self) -> &Chunks {
        &self.chunks
    }

    pub fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> &wgpu::Buffer {
        &self.index_buffer
    }

    pub fn num_indices(&self) -> u32 {
        self.num_indices
    }

    pub fn instance_buffer(&self) -> &wgpu::Buffer {
        &self.instance_buffer
    }

    pub fn num_instances(&self) -> u32 {
        self.instances.len() as u32
    }

    pub fn lights(&self) -> &Lights {
        &self.lights
    }
    pub fn lights_buffer(&self) -> &wgpu::Buffer {
        &self.lights_buffer
    }

    pub fn sky(&self) -> &Sky {
        &self.sky
    }

    pub fn update(&mut self, dt: Duration, player_position: &Vec3, device: &wgpu::Device) {
        self.chunks.update(player_position);
        self.create_instances(device);

        // move the sun and moon
        // TODO: precess slowly around Z too
        self.sun_offset = Quat::from_axis_angle(Vec3::Z, 0.02 * dt.as_secs_f32()) * self.sun_offset;
        self.moon_offset =
            Quat::from_axis_angle(Vec3::Z, 0.04 * dt.as_secs_f32()) * self.moon_offset;
        self.lights.lights[0].position = *player_position + self.sun_offset;
        self.lights.lights[1].position = *player_position + self.moon_offset;

        self.sky.update(dt, &self.lights.lights[0].position);
    }

    fn create_instances(&mut self, device: &wgpu::Device) {
        self.instances.clear();
        // TODO: only update chunks that are new, and drop instances that are no longer valid.
        for chunks in self.chunks.loaded().lock().expect("").values() {
            for chunk in chunks.iter() {
                for (x, row) in chunk.blocks().iter().enumerate() {
                    for (y, col) in row.iter().enumerate() {
                        for (z, block) in col.iter().enumerate() {
                            let block_position =
                                chunk.start() + Vec3::new(x as f32, y as f32, z as f32);
                            if block.is_active() {
                                self.instances
                                    .push(Instance::new(block_position, block.color()));
                            }
                        }
                    }
                }
            }
        }
        let instance_data = self
            .instances
            .iter()
            .map(Instance::to_raw)
            .collect::<Vec<_>>();

        // NOTE: we can't just write the buffer as the size of instance data may change from update
        // to update.
        self.instance_buffer.destroy();
        self.instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("instance buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
    }
}
