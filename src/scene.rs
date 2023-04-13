use std::time::Duration;

use crate::{
    block::Block,
    chunks::Chunks,
    instance::{Instance, RawInstance},
    light::Light,
    sky::Sky,
};
use glam::{Quat, Vec3};
use wgpu::util::DeviceExt;

pub struct Scene {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    chunks: Chunks,
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,

    sun: Light,
    sun_buffer: wgpu::Buffer,

    moon: Light,
    moon_buffer: wgpu::Buffer,

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
        let sun = Light::new(
            Vec3::new(0.0, 200.0, 50.0),
            wgpu::Color {
                r: 0.99,
                g: 0.85,
                b: 0.21,
                a: 1.0,
            },
        );
        let sun_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sun buffer"),
            contents: bytemuck::cast_slice(&[sun.to_raw()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let moon = Light::new(
            Vec3::new(0.0, -80.0, 40.0),
            wgpu::Color {
                r: 0.76,
                g: 0.77,
                b: 0.80,
                a: 1.0,
            },
        );
        let moon_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("moon buffer"),
            contents: bytemuck::cast_slice(&[moon.to_raw()]),
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

            sun,
            sun_buffer,
            moon,
            moon_buffer,
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

    pub fn sun(&self) -> &Light {
        &self.sun
    }

    pub fn sun_buffer(&self) -> &wgpu::Buffer {
        &self.sun_buffer
    }

    pub fn moon(&self) -> &Light {
        &self.moon
    }

    pub fn moon_buffer(&self) -> &wgpu::Buffer {
        &self.moon_buffer
    }

    pub fn sky(&self) -> &Sky {
        &self.sky
    }

    pub fn update(&mut self, dt: Duration, player_position: &Vec3, device: &wgpu::Device) {
        self.chunks.update(player_position);
        self.create_instances(device);

        // move the sun and moon
        // TODO: also precess around Z
        self.sun.position =
            Quat::from_axis_angle(Vec3::X, 0.05 * dt.as_secs_f32()) * self.sun.position;
        self.moon.position =
            Quat::from_axis_angle(Vec3::X, 0.1 * dt.as_secs_f32()) * self.moon.position;

        self.sky.update(dt, &self.sun.position);
    }

    fn create_instances(&mut self, device: &wgpu::Device) {
        self.instances.clear();
        // TODO: use new_chunks.
        //for key in self
        //    .chunks
        //    .new_chunks()
        //    .lock()
        //    .expect("lock new_chunks")
        //    .iter()
        //{
        //    for chunk in self
        //        .chunks
        //        .loaded()
        //        .lock()
        //        .expect("lock loaded")
        //        .get(key)
        //        .unwrap()
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
        //}
        let instance_data = self
            .instances
            .iter()
            .map(Instance::to_raw)
            .collect::<Vec<_>>();
        self.instance_buffer.destroy();
        self.instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("instance buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
    }
}
