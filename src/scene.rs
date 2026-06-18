use std::time::Duration;

use crate::{
    camera::Camera,
    chunks::Chunks,
    light::{Light, RawLight},
};
use bytemuck::{Pod, Zeroable};
use glam::{Quat, Vec3};

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct RawLights {
    lights: [RawLight; 2],
}

pub struct Lights {
    lights: Vec<Light>,
}

impl Lights {
    pub(crate) fn empty() -> Self {
        Self { lights: vec![] }
    }

    pub(crate) fn to_raw(&self) -> RawLights {
        let mut raw_lights = RawLights {
            lights: [RawLight::new(); 2],
        };
        for (idx, light) in self.lights.iter().enumerate() {
            raw_lights.lights[idx] = light.to_raw();
        }
        raw_lights
    }
}

pub struct Scene {
    chunks: Chunks,
    sun_offset: Vec3,
    moon_offset: Vec3,
    lights: Lights,
}

impl Scene {
    pub fn new(seed: u32, config: crate::config::Config) -> Self {
        let chunks = Chunks::new(seed, config.chunk_load_radius);

        // TODO: position sun relative to player always.
        let lights = Lights {
            lights: vec![
                // sun
                Light::new(
                    Vec3::new(0.0, 200.0, 50.0),
                    wgpu::Color { r: 0.99, g: 0.85, b: 0.21, a: 1.0 },
                ),
                // moon
                Light::new(
                    Vec3::new(0.0, -80.0, 40.0),
                    wgpu::Color { r: 0.76, g: 0.77, b: 0.80, a: 0.25 },
                ),
            ],
        };

        Self {
            sun_offset: Vec3::new(64.0, 64.0, 0.0),
            moon_offset: Vec3::new(-64.0, -64.0, 32.0),
            lights,
            chunks,
        }
    }

    pub fn chunks(&self) -> &Chunks {
        &self.chunks
    }

    pub fn sun_offset(&self) -> Vec3 {
        self.sun_offset
    }

    pub fn lights(&self) -> &Lights {
        &self.lights
    }

    pub fn update(&mut self, dt: Duration, camera: &Camera) {
        let player_position = camera.position();
        self.chunks.update(&player_position);

        let orbit_radius = camera.fog_end * 1.1;

        // move the sun and moon
        // TODO: precess slowly around Z too
        self.sun_offset = Quat::from_axis_angle(Vec3::Z, 0.02 * dt.as_secs_f32()) * self.sun_offset;
        self.moon_offset =
            Quat::from_axis_angle(Vec3::Z, 0.04 * dt.as_secs_f32()) * self.moon_offset;
            
        let current_sun_offset = self.sun_offset.normalize_or_zero() * orbit_radius;
        let current_moon_offset = self.moon_offset.normalize_or_zero() * orbit_radius;
            
        self.lights.lights[0].position = player_position + current_sun_offset;
        self.lights.lights[1].position = player_position + current_moon_offset;
    }
}
