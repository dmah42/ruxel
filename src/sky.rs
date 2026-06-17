use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;
use std::f32::consts::PI;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawSky {
    color: [f32; 4],
    a: [f32; 4],
    b: [f32; 4],
    c: [f32; 4],
    d: [f32; 4],
    e: [f32; 4],
    z: [f32; 4], // Yz, xz, yz, padding
    sun_dir: [f32; 4],
}

pub struct Sky {
    color: wgpu::Color,
    buffer: wgpu::Buffer,
    raw: RawSky,
}

impl Sky {
    pub fn new(device: &wgpu::Device) -> Self {
        let raw = RawSky {
            color: [0.0; 4],
            a: [0.0; 4],
            b: [0.0; 4],
            c: [0.0; 4],
            d: [0.0; 4],
            e: [0.0; 4],
            z: [0.0; 4],
            sun_dir: [0.0; 4],
        };
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sky buffer"),
            contents: bytemuck::cast_slice(&[raw]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self { color: wgpu::Color::BLACK, buffer, raw }
    }

    pub fn color(&self) -> wgpu::Color {
        self.color
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn to_raw(&self, _is_srgb_surface: bool) -> RawSky {
        self.raw
    }

    pub fn update(&mut self, _dt: Duration, sun_dir: &Vec3) {
        let sun_dir = sun_dir.normalize();
        let turbidity = 2.5f32;

        let solar_zenith = sun_dir.y.clamp(-1.0, 1.0).acos();
        
        let sky_day = glam::Vec3::new(135.0 / 255.0, 206.0 / 255.0, 235.0 / 255.0);
        let sky_night = glam::Vec3::new(12.0 / 255.0, 20.0 / 255.0, 69.0 / 255.0);
        let frac = sun_dir.y.max(0.0);
        let color = sky_day * frac + sky_night * (1.0 - frac);

        let yz_num = (4.0453 * turbidity - 4.9710) * ((4.0 / 9.0 - turbidity / 120.0) * (PI - 2.0 * solar_zenith)).tan() - 0.2155 * turbidity + 2.4192;
        let y0_den = (4.0453 * turbidity - 4.9710) * ((4.0 / 9.0 - turbidity / 120.0) * PI).tan() - 0.2155 * turbidity + 2.4192;
        let yz = yz_num / y0_den;

        let z3 = solar_zenith.powi(3);
        let z2 = solar_zenith.powi(2);
        let z = solar_zenith;
        let t_vec = Vec3::new(turbidity * turbidity, turbidity, 1.0);

        let x = Vec3::new(
            0.00166 * z3 - 0.00375 * z2 + 0.00209 * z,
            -0.02903 * z3 + 0.06377 * z2 - 0.03202 * z + 0.00394,
            0.11693 * z3 - 0.21196 * z2 + 0.06052 * z + 0.25886,
        );
        let xz = t_vec.dot(x);

        let y = Vec3::new(
            0.00275 * z3 - 0.00610 * z2 + 0.00317 * z,
            -0.04214 * z3 + 0.08970 * z2 - 0.04153 * z + 0.00516,
            0.15346 * z3 - 0.26756 * z2 + 0.06670 * z + 0.26688,
        );
        let y_z = t_vec.dot(y);

        let a_y = 0.1787 * turbidity - 1.4630;
        let b_y = -0.3554 * turbidity + 0.4275;
        let c_y = -0.0227 * turbidity + 5.3251;
        let d_y = 0.1206 * turbidity - 2.5771;
        let e_y = -0.0670 * turbidity + 0.3703;

        let a_x = -0.0193 * turbidity - 0.2592;
        let b_x = -0.0665 * turbidity + 0.0008;
        let c_x = -0.0004 * turbidity + 0.2122;
        let d_x = -0.0641 * turbidity - 0.8989;
        let e_x = -0.0033 * turbidity + 0.0452;

        let a_y2 = -0.0167 * turbidity - 0.2608;
        let b_y2 = -0.0950 * turbidity + 0.0092;
        let c_y2 = -0.0079 * turbidity + 0.2102;
        let d_y2 = -0.0441 * turbidity - 1.6537;
        let e_y2 = -0.0109 * turbidity + 0.0529;

        self.raw = RawSky {
            color: [color.x, color.y, color.z, 1.0],
            a: [a_y, a_x, a_y2, 0.0],
            b: [b_y, b_x, b_y2, 0.0],
            c: [c_y, c_x, c_y2, 0.0],
            d: [d_y, d_x, d_y2, 0.0],
            e: [e_y, e_x, e_y2, 0.0],
            z: [yz, xz, y_z, 0.0],
            sun_dir: [sun_dir.x, sun_dir.y, sun_dir.z, 0.0],
        };
    }
}
