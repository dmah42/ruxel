use std::{cmp::max, time::Duration};

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawSky {
    color: [f32; 4],
}

pub struct Sky {
    color: wgpu::Color,
    buffer: wgpu::Buffer,
}

impl Sky {
    pub fn new(device: &wgpu::Device) -> Self {
        let color = wgpu::Color::BLACK;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sky buffer"),
            contents: bytemuck::cast_slice(&[[
                color.r as f32,
                color.g as f32,
                color.b as f32,
                1.0,
            ]]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self { color, buffer }
    }

    pub fn color(&self) -> wgpu::Color {
        self.color
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn to_raw(&self, is_srgb_surface: bool) -> [f32; 4] {
        if is_srgb_surface {
            let lin = palette::Srgb::new(
                self.color.r as f32,
                self.color.g as f32,
                self.color.b as f32,
            )
            .into_linear();
            [lin.red, lin.green, lin.blue, 1.0]
        } else {
            [
                self.color.r as f32,
                self.color.g as f32,
                self.color.b as f32,
                1.0,
            ]
        }
    }

    pub fn update(&mut self, _dt: Duration, sun_position: &Vec3) {
        let sky_day = wgpu::Color {
            r: 135.0 / 255.0,
            g: 206.0 / 255.0,
            b: 235.0 / 255.0,
            a: 1.0,
        };
        let sky_night = wgpu::Color {
            r: 12.0 / 255.0,
            g: 20.0 / 255.0,
            b: 69.0 / 255.0,
            a: 1.0,
        };
        
        let frac = (max(0, sun_position.y as i32) as f64) / 200.0;
        let frac = frac.clamp(0.0, 1.0);
        
        self.color = wgpu::Color {
            r: sky_day.r * frac + sky_night.r * (1.0 - frac),
            g: sky_day.g * frac + sky_night.g * (1.0 - frac),
            b: sky_day.b * frac + sky_night.b * (1.0 - frac),
            a: 1.0,
        };
    }
}
