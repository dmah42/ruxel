use crate::vertex::Vertex;

#[derive(Debug, Copy, Clone)]
pub enum Type {
    Inactive,
    Sand,
    Grass,
    Rock,
    Ice,
}

const fn vertex(pos: [i8; 3], n: [f32; 3]) -> Vertex {
    Vertex::new([pos[0] as f32, pos[1] as f32, pos[2] as f32], n)
}

#[derive(Debug, Copy, Clone)]
pub struct Block {
    ty: Type,
}

impl Block {
    pub fn new() -> Self {
        Self { ty: Type::Inactive }
    }

    pub fn set_type(&mut self, ty: Type) {
        self.ty = ty;
    }

    pub fn is_active(&self) -> bool {
        !matches!(self.ty, Type::Inactive)
    }

    pub fn color(&self) -> wgpu::Color {
        match self.ty {
            Type::Ice => wgpu::Color::WHITE,
            Type::Rock => wgpu::Color {
                r: 0.2,
                g: 0.2,
                b: 0.2,
                a: 1.0,
            },
            Type::Grass => wgpu::Color::GREEN,
            Type::Sand => wgpu::Color {
                r: 0.50,
                g: 0.80,
                b: 0.16,
                a: 1.0,
            },
            Type::Inactive => wgpu::Color::TRANSPARENT,
        }
    }

    pub const VERTICES: &[Vertex] = &[
        // top (0, 0, 1)
        vertex([0, 0, 1], [0.0, 0.0, 1.0]),
        vertex([1, 0, 1], [0.0, 0.0, 1.0]),
        vertex([1, 1, 1], [0.0, 0.0, 1.0]),
        vertex([0, 1, 1], [0.0, 0.0, 1.0]),
        // bottom (0, 0, -1)
        vertex([0, 1, 0], [0.0, 0.0, -1.0]),
        vertex([1, 1, 0], [0.0, 0.0, -1.0]),
        vertex([1, 0, 0], [0.0, 0.0, -1.0]),
        vertex([0, 0, 0], [0.0, 0.0, -1.0]),
        // right (1, 0, 0)
        vertex([1, 0, 0], [1.0, 0.0, 0.0]),
        vertex([1, 1, 0], [1.0, 0.0, 0.0]),
        vertex([1, 1, 1], [1.0, 0.0, 0.0]),
        vertex([1, 0, 1], [1.0, 0.0, 0.0]),
        // left (-1, 0, 0)
        vertex([0, 0, 1], [-1.0, 0.0, 0.0]),
        vertex([0, 1, 1], [-1.0, 0.0, 0.0]),
        vertex([0, 1, 0], [-1.0, 0.0, 0.0]),
        vertex([0, 0, 0], [-1.0, 0.0, 0.0]),
        // front (0, 1, 0)
        vertex([1, 1, 0], [0.0, 1.0, 0.0]),
        vertex([0, 1, 0], [0.0, 1.0, 0.0]),
        vertex([0, 1, 1], [0.0, 1.0, 0.0]),
        vertex([1, 1, 1], [0.0, 1.0, 0.0]),
        // back (0, -1, 0)
        vertex([1, 0, 1], [0.0, -1.0, 0.0]),
        vertex([0, 0, 1], [0.0, -1.0, 0.0]),
        vertex([0, 0, 0], [0.0, -1.0, 0.0]),
        vertex([1, 0, 0], [0.0, -1.0, 0.0]),
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
