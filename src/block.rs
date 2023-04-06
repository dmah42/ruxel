use crate::vertex::Vertex;

#[derive(Debug, Copy, Clone)]
pub enum Type {
    Inactive,
    Grass,
}

const fn vertex(pos: [i8; 3], n: [f32; 3]) -> Vertex {
    Vertex::new([pos[0] as f32, pos[1] as f32, pos[2] as f32], n)
}

#[derive(Debug, Copy, Clone)]
pub struct Block {
    pub ty: Type,
}

impl Block {
    pub fn new(ty: Type) -> Self {
        Self { ty }
    }

    pub const VERTICES: &[Vertex] = &[
        // top (0, 0, 1)
        vertex([-1, -1, 1], [0.0, 0.0, 1.0]),
        vertex([1, -1, 1], [0.0, 0.0, 1.0]),
        vertex([1, 1, 1], [0.0, 0.0, 1.0]),
        vertex([-1, 1, 1], [0.0, 0.0, 1.0]),
        // bottom (0, 0, -1)
        vertex([-1, 1, -1], [0.0, 0.0, -1.0]),
        vertex([1, 1, -1], [0.0, 0.0, -1.0]),
        vertex([1, -1, -1], [0.0, 0.0, -1.0]),
        vertex([-1, -1, -1], [0.0, 0.0, -1.0]),
        // right (1, 0, 0)
        vertex([1, -1, -1], [1.0, 0.0, 0.0]),
        vertex([1, 1, -1], [1.0, 0.0, 0.0]),
        vertex([1, 1, 1], [1.0, 0.0, 0.0]),
        vertex([1, -1, 1], [1.0, 0.0, 0.0]),
        // left (-1, 0, 0)
        vertex([-1, -1, 1], [-1.0, 0.0, 0.0]),
        vertex([-1, 1, 1], [-1.0, 0.0, 0.0]),
        vertex([-1, 1, -1], [-1.0, 0.0, 0.0]),
        vertex([-1, -1, -1], [-1.0, 0.0, 0.0]),
        // front (0, 1, 0)
        vertex([1, 1, -1], [0.0, 1.0, 0.0]),
        vertex([-1, 1, -1], [0.0, 1.0, 0.0]),
        vertex([-1, 1, 1], [0.0, 1.0, 0.0]),
        vertex([1, 1, 1], [0.0, 1.0, 0.0]),
        // back (0, -1, 0)
        vertex([1, -1, 1], [0.0, -1.0, 0.0]),
        vertex([-1, -1, 1], [0.0, -1.0, 0.0]),
        vertex([-1, -1, -1], [0.0, -1.0, 0.0]),
        vertex([1, -1, -1], [0.0, -1.0, 0.0]),
    ];

    pub const INDICES: &[u16] = &[
        0, 1, 2, 2, 3, 0, // top
        4, 5, 6, 6, 7, 4, // bottom
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // front
        20, 21, 22, 22, 23, 20, // back
    ];
}
