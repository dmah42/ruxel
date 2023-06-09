use bytemuck::{Pod, Zeroable};
use glam::Vec3;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawLight {
    position: [f32; 3],
    _padding: u32,
    color: [f32; 4],
}

impl RawLight {
    pub fn new() -> Self {
        Self {
            position: [0.0; 3],
            _padding: 0,
            color: [0.0; 4],
        }
    }
}

pub struct Light {
    pub position: Vec3,
    color: wgpu::Color,
}

impl Light {
    pub fn new(position: Vec3, color: wgpu::Color) -> Self {
        Self { position, color }
    }

    pub fn to_raw(&self) -> RawLight {
        RawLight {
            position: [self.position.x, self.position.y, self.position.z],
            _padding: 0,
            color: [
                self.color.r as f32,
                self.color.g as f32,
                self.color.b as f32,
                self.color.a as f32,
            ],
        }
    }
}
