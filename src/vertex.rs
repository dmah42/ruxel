use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
// TODO RawVertex to make the vertex ctor a bit nicer.
pub struct Vertex {
    position: [f32; 3],
    color: [u8; 4],
    normal_and_ao: [i8; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Unorm8x4, 2 => Snorm8x4];

    pub const fn new(position: [f32; 3], color: [u8; 4], normal_and_ao: [i8; 4]) -> Self {
        Vertex {
            position,
            color,
            normal_and_ao,
        }
    }

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SimpleVertex {
    position: [f32; 3],
    normal: [i8; 4],
}

impl SimpleVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Snorm8x4];

    pub const fn new(position: [f32; 3], normal: [i8; 4]) -> Self {
        SimpleVertex { position, normal }
    }

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SimpleVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

const fn simple_vertex(pos: [i8; 3], n: [f32; 3]) -> SimpleVertex {
    let nx = (n[0] * 127.0) as i8;
    let ny = (n[1] * 127.0) as i8;
    let nz = (n[2] * 127.0) as i8;
    SimpleVertex::new(
        [pos[0] as f32, pos[1] as f32, pos[2] as f32],
        [nx, ny, nz, 0],
    )
}

pub const CUBE_VERTICES: &[SimpleVertex] = &[
    // top (0, 0, 1)
    simple_vertex([0, 0, 1], [0.0, 0.0, 1.0]),
    simple_vertex([1, 0, 1], [0.0, 0.0, 1.0]),
    simple_vertex([1, 1, 1], [0.0, 0.0, 1.0]),
    simple_vertex([0, 1, 1], [0.0, 0.0, 1.0]),
    // bottom (0, 0, -1)
    simple_vertex([0, 1, 0], [0.0, 0.0, -1.0]),
    simple_vertex([1, 1, 0], [0.0, 0.0, -1.0]),
    simple_vertex([1, 0, 0], [0.0, 0.0, -1.0]),
    simple_vertex([0, 0, 0], [0.0, 0.0, -1.0]),
    // right (1, 0, 0)
    simple_vertex([1, 0, 0], [1.0, 0.0, 0.0]),
    simple_vertex([1, 1, 0], [1.0, 0.0, 0.0]),
    simple_vertex([1, 1, 1], [1.0, 0.0, 0.0]),
    simple_vertex([1, 0, 1], [1.0, 0.0, 0.0]),
    // left (-1, 0, 0)
    simple_vertex([0, 0, 1], [-1.0, 0.0, 0.0]),
    simple_vertex([0, 1, 1], [-1.0, 0.0, 0.0]),
    simple_vertex([0, 1, 0], [-1.0, 0.0, 0.0]),
    simple_vertex([0, 0, 0], [-1.0, 0.0, 0.0]),
    // front (0, 1, 0)
    simple_vertex([1, 1, 0], [0.0, 1.0, 0.0]),
    simple_vertex([0, 1, 0], [0.0, 1.0, 0.0]),
    simple_vertex([0, 1, 1], [0.0, 1.0, 0.0]),
    simple_vertex([1, 1, 1], [0.0, 1.0, 0.0]),
    // back (0, -1, 0)
    simple_vertex([1, 0, 1], [0.0, -1.0, 0.0]),
    simple_vertex([0, 0, 1], [0.0, -1.0, 0.0]),
    simple_vertex([0, 0, 0], [0.0, -1.0, 0.0]),
    simple_vertex([1, 0, 0], [0.0, -1.0, 0.0]),
];

pub const CUBE_INDICES: &[u16] = &[
    0, 1, 2, 2, 3, 0, // top
    4, 5, 6, 6, 7, 4, // bottom
    8, 9, 10, 10, 11, 8, // right
    12, 13, 14, 14, 15, 12, // left
    16, 17, 18, 18, 19, 16, // front
    20, 21, 22, 22, 23, 20, // back
];

pub const WIREFRAME_INDICES: &[u16] = &[
    0, 1, 1, 2, 2, 3, 3, 0, // top
    4, 5, 5, 6, 6, 7, 7, 4, // bottom
    0, 7, 1, 6, 2, 5, 3, 4, // sides
];
