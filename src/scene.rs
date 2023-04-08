use std::time::Duration;

use crate::{
    block::{self, Block},
    instance::Instance,
    light::Light,
    terrain,
};
use glam::{Quat, Vec3};
use noise::NoiseFn;
use wgpu::util::DeviceExt;

#[derive(Debug)]
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

        let light = Light::new(Vec3::new(100.0, 5000.0, 50.0), wgpu::Color::WHITE);
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
            Quat::from_axis_angle(Vec3::Y, 0.05 * dt.as_secs_f32()) * old_light_position;
    }

    fn create_chunks() -> Vec<Chunk> {
        let terrain = &terrain::gen(0);

        let chunks = {
            (0..4)
                .flat_map(|cx| {
                    (0..4).flat_map(move |cy| {
                        (0..4).map(move |cz| {
                            let mut chunk = Chunk {
                                blocks: [[[Block::new(); 16]; 16]; 16],
                                start: Vec3::new(
                                    16.0 * (cx as f32),
                                    16.0 * (cy as f32),
                                    16.0 * (cz as f32),
                                ),
                            };
                            println!("chunk ({cx},{cy},{cz}) starting at {:?}", chunk.start);
                            for (x, row) in chunk.blocks.iter_mut().enumerate() {
                                for (y, col) in row.iter_mut().enumerate() {
                                    for (z, block) in col.iter_mut().enumerate() {
                                        let blockx = x + (16 * cx);
                                        let blocky = y + (16 * cy);
                                        let blockz = z + (16 * cz);
                                        let point: [f64; 2] =
                                            [blockx as f64 / 64.0, blockz as f64 / 64.0];
                                        let height =
                                            ((terrain.get(point) + 1.0) * 64.0 / 2.0) as f32;
                                        if (blocky as f32) < 32.0 {
                                            block.set_type(block::Type::Water);
                                        }
                                        if (blocky as f32) < height {
                                            block.set_type(match blocky {
                                                0..=35 => block::Type::Sand,
                                                36..=48 => block::Type::Grass,
                                                49..=55 => block::Type::Rock,
                                                56..=64 => block::Type::Ice,
                                                _ => panic!("unexpected height"),
                                            });
                                        }
                                    }
                                }
                            }
                            chunk
                        })
                    })
                })
                .collect::<Vec<_>>()
            //vec![chunk]
        };
        chunks
    }

    fn create_instances(chunks: &Vec<Chunk>) -> Vec<Instance> {
        let mut instances: Vec<Instance> = vec![];
        for chunk in chunks.iter() {
            for (x, row) in chunk.blocks.iter().enumerate() {
                for (y, col) in row.iter().enumerate() {
                    for (z, block) in col.iter().enumerate() {
                        let block_position = chunk.start + Vec3::new(x as f32, y as f32, z as f32);
                        if block.is_active() {
                            instances.push(Instance::new(block_position, block.color()));
                        }
                    }
                }
            }
        }
        instances
    }
}
