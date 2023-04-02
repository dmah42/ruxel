use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub const TRIANGLE: &[Vertex] = &[
    Vertex {
        position: glam::Vec3::new(0.0, 0.5, 0.0).to_array(),
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: glam::Vec3::new(-0.5, -0.5, 0.0).to_array(),
        color: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: glam::Vec3::new(0.5, -0.5, 0.0).to_array(),
        color: [0.0, 0.0, 1.0],
    },
];
