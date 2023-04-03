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

pub const PENTAGON: &[Vertex] = &[
    Vertex {
        position: [-0.0868241, 0.49240386, 0.0],
        color: [0.5, 0.0, 0.0],
    }, // A
    Vertex {
        position: [-0.49513406, 0.06958647, 0.0],
        color: [0.5, 0.5, 0.0],
    }, // B
    Vertex {
        position: [-0.21918549, -0.44939706, 0.0],
        color: [0.0, 0.5, 0.0],
    }, // C
    Vertex {
        position: [0.35966998, -0.3473291, 0.0],
        color: [0.0, 0.5, 0.5],
    }, // D
    Vertex {
        position: [0.44147372, 0.2347359, 0.0],
        color: [0.0, 0.0, 0.5],
    }, // E
];

pub const PENTAGON_INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];
