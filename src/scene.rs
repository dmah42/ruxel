use crate::{instance::Instance, vertex::Vertex};
use wgpu::util::DeviceExt;

pub struct Scene {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
}

impl Scene {
    pub fn new(device: &wgpu::Device) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex buffer"),
            contents: bytemuck::cast_slice(CUBE),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index buffer"),
            contents: bytemuck::cast_slice(CUBE_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = CUBE_INDICES.len() as u32;

        const NUM_CUBES_PER_ROW: u32 = 10;

        let instances = (0..NUM_CUBES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_CUBES_PER_ROW).map(move |x| {
                    let start = glam::Vec3::new(
                        NUM_CUBES_PER_ROW as f32 * -1.0,
                        -0.5,
                        NUM_CUBES_PER_ROW as f32 * -1.0,
                    );
                    let position = start
                        + glam::Vec3::new(x as f32 * 2.0, ((x + z) as f32).sin(), z as f32 * 2.0);
                    let rotation = glam::Quat::from_axis_angle(glam::Vec3::Z, 0.0);

                    Instance::new(position, rotation)
                })
            })
            .collect::<Vec<_>>();

        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("instance buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            num_indices,

            instances,
            instance_buffer,
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
}

const fn vertex(pos: [i8; 3], c: [f32; 3]) -> Vertex {
    Vertex::new([pos[0] as f32, pos[1] as f32, pos[2] as f32], c)
}

const GRASSES: [[f32; 3]; 6] = [
    [0.604, 0.804, 0.196],
    [0.655, 0.804, 0.196],
    [0.706, 0.804, 0.196],
    [0.553, 0.804, 0.196],
    [0.502, 0.804, 0.196],
    [0.451, 0.804, 0.196],
];

const CUBE: &[Vertex] = &[
    // top (0, 0, 1)
    vertex([-1, -1, 1], GRASSES[1]),
    vertex([1, -1, 1], GRASSES[0]),
    vertex([1, 1, 1], GRASSES[2]),
    vertex([-1, 1, 1], GRASSES[3]),
    // bottom (0, 0, -1)
    vertex([-1, 1, -1], GRASSES[4]),
    vertex([1, 1, -1], GRASSES[5]),
    vertex([1, -1, -1], GRASSES[0]),
    vertex([-1, -1, -1], GRASSES[2]),
    // right (1, 0, 0)
    vertex([1, -1, -1], GRASSES[0]),
    vertex([1, 1, -1], GRASSES[5]),
    vertex([1, 1, 1], GRASSES[2]),
    vertex([1, -1, 1], GRASSES[0]),
    // left (-1, 0, 0)
    vertex([-1, -1, 1], GRASSES[1]),
    vertex([-1, 1, 1], GRASSES[3]),
    vertex([-1, 1, -1], GRASSES[4]),
    vertex([-1, -1, -1], GRASSES[2]),
    // front (0, 1, 0)
    vertex([1, 1, -1], GRASSES[5]),
    vertex([-1, 1, -1], GRASSES[4]),
    vertex([-1, 1, 1], GRASSES[3]),
    vertex([1, 1, 1], GRASSES[2]),
    // back (0, -1, 0)
    vertex([1, -1, 1], GRASSES[0]),
    vertex([-1, -1, 1], GRASSES[1]),
    vertex([-1, -1, -1], GRASSES[2]),
    vertex([1, -1, -1], GRASSES[0]),
];

const CUBE_INDICES: &[u16] = &[
    0, 1, 2, 2, 3, 0, // top
    4, 5, 6, 6, 7, 4, // bottom
    8, 9, 10, 10, 11, 8, // right
    12, 13, 14, 14, 15, 12, // left
    16, 17, 18, 18, 19, 16, // front
    20, 21, 22, 22, 23, 20, // back
];
