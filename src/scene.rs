use std::time::Duration;

use crate::{
    block::{self, Block},
    instance::Instance,
    light::Light,
};
use glam::{Quat, Vec3};
use rand::Rng;
use wgpu::util::DeviceExt;

pub struct Chunk {
    blocks: [[[Block; 16]; 16]; 16],
    start: Vec3,
}

pub struct Scene {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    chunks: Vec<Chunk>,
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,

    light: Light,
    light_buffer: wgpu::Buffer,
}

impl Scene {
    pub fn new(device: &wgpu::Device) -> Self {
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

        let chunks = Self::create_chunks();
        let instances = Self::create_instances(&chunks);

        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("instance buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let light = Light::new(Vec3::new(100.0, 2.0, 50.0), wgpu::Color::WHITE);
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("light buffer"),
            contents: bytemuck::cast_slice(&[light.to_raw()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            vertex_buffer,
            index_buffer,
            num_indices,

            instances,
            instance_buffer,

            light,
            light_buffer,

            chunks,
        }
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

    pub fn light(&self) -> &Light {
        &self.light
    }

    pub fn light_buffer(&self) -> &wgpu::Buffer {
        &self.light_buffer
    }

    pub fn update(&mut self, dt: Duration) {
        // move the light
        let old_light_position = self.light.position;
        self.light.position =
            Quat::from_axis_angle(Vec3::Y, 0.2 * dt.as_secs_f32()) * old_light_position;
    }

    fn create_chunks() -> Vec<Chunk> {
        let mut rng = rand::thread_rng();
        (0..4)
            .map(|idx| {
                let mut chunk = Chunk {
                    blocks: [[[Block::new(block::Type::Inactive); 16]; 16]; 16],
                    start: Vec3::new(-8.0 * (idx / 2) as f32, -8.0, -8.0 * (idx % 2) as f32),
                };
                for row in chunk.blocks.iter_mut() {
                    for col in row.iter_mut() {
                        for block in col.iter_mut() {
                            if rng.gen_bool(0.1) {
                                block.ty = block::Type::Grass;
                            }
                        }
                    }
                }
                chunk
            })
            .collect::<Vec<_>>()
    }

    fn create_instances(chunks: &Vec<Chunk>) -> Vec<Instance> {
        let mut instances: Vec<Instance> = vec![];
        for chunk in chunks.iter() {
            let mut position = chunk.start;
            chunk.blocks.map(|blockz| {
                blockz.map(|blocky| {
                    blocky.map(|block| {
                        let color = match block.ty {
                            block::Type::Grass => wgpu::Color::GREEN,
                            block::Type::Inactive => wgpu::Color::TRANSPARENT,
                        };
                        if color != wgpu::Color::TRANSPARENT {
                            instances.push(Instance::new(position, color));
                        }
                        position.x += 1.0;
                    });
                    position.y += 1.0;
                    position.x = chunk.start.x;
                });
                position.z += 1.0;
                position.y = chunk.start.y;
            });
        }
        instances
    }
}
