#[derive(Debug, Copy, Clone)]
pub enum Type {
    Inactive = 0,
    Sand = 1,
    Grass = 2,
    Rock = 3,
    Ice = 4,
    Water = 5,
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

    pub fn is_solid(&self) -> bool {
        !matches!(self.ty, Type::Inactive | Type::Water)
    }

    pub fn color(&self) -> wgpu::Color {
        match self.ty {
            Type::Ice => wgpu::Color {
                r: 0.8,
                g: 0.9,
                b: 1.0,
                a: 1.0,
            },
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
            Type::Water => wgpu::Color {
                r: 0.0,
                g: 0.2,
                b: 1.0,
                a: 0.5,
            },
            Type::Inactive => wgpu::Color::TRANSPARENT,
        }
    }

    pub fn material_id(&self) -> u32 {
        self.ty as u32
    }
}
