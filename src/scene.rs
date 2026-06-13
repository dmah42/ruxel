use std::time::Duration;

use crate::{
    chunks::Chunks,
    light::{Light, RawLight},
    sky::Sky,
    vertex::{CUBE_INDICES, CUBE_VERTICES},
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

pub struct ChunkBuffers {
    pub vertex_buffer: wgpu::Buffer,
    pub opaque_index_buffer: Option<(wgpu::Buffer, u32)>,
    pub transparent_index_buffer: Option<(wgpu::Buffer, u32)>,
}

pub struct Scene {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    chunks: Chunks,
    chunk_buffers: std::collections::HashMap<glam::UVec2, Vec<Option<ChunkBuffers>>>,

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
            contents: bytemuck::cast_slice(CUBE_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index buffer"),
            contents: bytemuck::cast_slice(CUBE_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = CUBE_INDICES.len() as u32;

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

        let chunk_buffers = std::collections::HashMap::new();

        Self {
            vertex_buffer,
            index_buffer,
            num_indices,

            chunk_buffers,

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

    pub fn chunk_buffers(
        &self,
    ) -> &std::collections::HashMap<glam::UVec2, Vec<Option<ChunkBuffers>>> {
        &self.chunk_buffers
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
        self.update_chunk_buffers(device);

        // move the sun and moon
        // TODO: precess slowly around Z too
        self.sun_offset = Quat::from_axis_angle(Vec3::Z, 0.02 * dt.as_secs_f32()) * self.sun_offset;
        self.moon_offset =
            Quat::from_axis_angle(Vec3::Z, 0.04 * dt.as_secs_f32()) * self.moon_offset;
        self.lights.lights[0].position = *player_position + self.sun_offset;
        self.lights.lights[1].position = *player_position + self.moon_offset;

        self.sky.update(dt, &self.lights.lights[0].position);
    }

    fn update_chunk_buffers(&mut self, device: &wgpu::Device) {
        let loaded = self.chunks.loaded();
        let mut locked_loaded = loaded.lock().expect("");

        // Remove buffers for chunks that are no longer loaded
        self.chunk_buffers
            .retain(|key, _| locked_loaded.contains_key(key));

        let terrain = self.chunks.terrain().clone();

        // 1. Find all dirty chunks
        let mut dirty_chunks = Vec::new();
        for (key, chunks) in locked_loaded.iter() {
            for (i, chunk) in chunks.iter().enumerate() {
                if chunk.dirty() {
                    dirty_chunks.push((*key, i));
                }
            }
        }

        // 2. Build meshes with read-only access to all chunks
        let mut new_meshes = Vec::new();
        for (key, i) in dirty_chunks {
            let chunk = &locked_loaded.get(&key).unwrap()[i];
            let mesh = crate::mesh::ChunkMesh::build(chunk, &locked_loaded, &terrain);
            new_meshes.push((key, i, mesh));
        }

        // 3. Apply the built meshes and mark clean
        for (key, i, mesh) in new_meshes {
            let chunk = &mut locked_loaded.get_mut(&key).unwrap()[i];
            chunk.set_mesh(mesh);
            chunk.clean();
        }

        // 4. Update GPU buffers
        for (key, chunks) in locked_loaded.iter_mut() {
            let col_buffers = self
                .chunk_buffers
                .entry(*key)
                .or_insert_with(|| (0..chunks.len()).map(|_| None).collect());

            for (i, chunk) in chunks.iter_mut().enumerate() {
                if let Some(mesh) = chunk.take_mesh() {
                    let opaque_indices = mesh.opaque_indices();
                    let transparent_indices = mesh.transparent_indices();

                    if opaque_indices.is_empty() && transparent_indices.is_empty() {
                        col_buffers[i] = None;
                        continue;
                    }

                    let vertex_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("chunk vertex buffer"),
                            contents: bytemuck::cast_slice(mesh.vertices()),
                            usage: wgpu::BufferUsages::VERTEX,
                        });

                    let opaque_index_buffer = if !opaque_indices.is_empty() {
                        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("chunk opaque index buffer"),
                            contents: bytemuck::cast_slice(opaque_indices),
                            usage: wgpu::BufferUsages::INDEX,
                        });
                        Some((buf, opaque_indices.len() as u32))
                    } else {
                        None
                    };

                    let transparent_index_buffer = if !transparent_indices.is_empty() {
                        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
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
            }
        }
    }
}
