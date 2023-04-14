use std::{cmp::max, time::Duration};

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawSky {
    color: [f32; 3],
}

pub struct Sky {
    color: wgpu::Color,
    buffer: wgpu::Buffer,
}

// TODO: https://nicoschertler.wordpress.com/2013/04/03/simulating-a-days-sky/
impl Sky {
    pub fn new(device: &wgpu::Device) -> Self {
        let color = wgpu::Color::BLACK;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sky buffer"),
            contents: bytemuck::cast_slice(&[[color.r, color.g, color.b]]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        Self { color, buffer }
    }

    pub fn color(&self) -> wgpu::Color {
        self.color
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn to_raw(&self) -> [f32; 3] {
        [
            self.color.r as f32,
            self.color.g as f32,
            self.color.b as f32,
        ]
    }

    pub fn update(&mut self, _dt: Duration, sun_position: &Vec3) {
        const SKY_DAY: wgpu::Color = wgpu::Color {
            r: 135.0 / 255.0,
            g: 206.0 / 255.0,
            b: 235.0 / 255.0,
            a: 1.0,
        };
        const SKY_NIGHT: wgpu::Color = wgpu::Color {
            r: 12.0 / 255.0,
            g: 20.0 / 255.0,
            b: 69.0 / 255.0,
            a: 1.0,
        };
        let frac = (max(0, sun_position.y as i32) as f64) / 200.0;
        self.color = wgpu::Color {
            r: (SKY_DAY.r * frac + SKY_NIGHT.r * (1.0 - frac)),
            g: (SKY_DAY.g * frac + SKY_NIGHT.g * (1.0 - frac)),
            b: (SKY_DAY.b * frac + SKY_NIGHT.b * (1.0 - frac)),
            a: 1.0,
        };
    }
}
