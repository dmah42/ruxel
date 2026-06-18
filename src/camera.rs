use std::{f32::consts::FRAC_PI_2, time::Duration};

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseScrollDelta},
    keyboard::KeyCode,
};

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.001;
const GRAVITY: f32 = 25.0;
const PLAYER_HEIGHT: f32 = 2.6;
const SPEED: f32 = 4.0;
const SENSITIVITY: f32 = 0.4;
const JUMP_VELOCITY: f32 = 10.0;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Uniform {
    view_proj: [f32; 16],
    inv_view_proj: [f32; 16],
    view_pos: [f32; 4],
    fog_start_sq: f32,
    fog_end_sq: f32,
    _padding: [f32; 2],
}

impl Uniform {
    pub fn new() -> Self {
        Self {
            view_proj: *Mat4::IDENTITY.as_ref(),
            inv_view_proj: *Mat4::IDENTITY.as_ref(),
            view_pos: [0.0; 4],
            fog_start_sq: 0.0,
            fog_end_sq: 0.0,
            _padding: [0.0; 2],
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera, load_radius: i32) {
        let vp = camera.projection.matrix() * camera.matrix();
        self.view_proj = *vp.as_ref();
        self.inv_view_proj = *vp.inverse().as_ref();
        let visual_pos = camera.visual_position();
        self.view_pos = [visual_pos.x, visual_pos.y, visual_pos.z, 1.0];

        let fog_start = (load_radius as f32 - 0.5) * 16.0;
        let fog_end = load_radius as f32 * 16.0;
        self.fog_start_sq = fog_start * fog_start;
        self.fog_end_sq = fog_end * fog_end;
    }
}

#[derive(Debug)]
pub struct Projection {
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new(aspect: f32, fovy: f32, znear: f32, zfar: f32) -> Self {
        Self {
            aspect,
            fovy,
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[derive(Debug)]
pub struct Camera {
    position: Vec3,
    yaw: f32,
    pitch: f32,
    velocity: Vec3,
    step_offset: f32,
    pub projection: Projection,
}

impl Camera {
    pub fn new(position: Vec3, yaw: f32, pitch: f32, projection: Projection) -> Self {
        Self {
            position,
            yaw,
            pitch,
            velocity: Vec3::ZERO,
            step_offset: 0.0,
            projection,
        }
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub fn visual_position(&self) -> Vec3 {
        let mut p = self.position;
        p.y += self.step_offset;
        p
    }

    pub fn forward(&self) -> Vec3 {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize()
    }

    fn matrix(&self) -> Mat4 {
        Mat4::look_to_rh(self.visual_position(), self.forward(), Vec3::Y)
    }

    pub fn raycast(
        &self,
        chunks: &crate::chunks::Chunks,
        max_distance: f32,
    ) -> Option<(glam::IVec3, glam::IVec3)> {
        let origin = self.visual_position();
        let dir = self.forward();

        let mut x = origin.x.floor() as i32;
        let mut y = origin.y.floor() as i32;
        let mut z = origin.z.floor() as i32;

        let step_x = dir.x.signum() as i32;
        let step_y = dir.y.signum() as i32;
        let step_z = dir.z.signum() as i32;

        let t_delta_x = if dir.x != 0.0 {
            (1.0 / dir.x).abs()
        } else {
            f32::INFINITY
        };
        let t_delta_y = if dir.y != 0.0 {
            (1.0 / dir.y).abs()
        } else {
            f32::INFINITY
        };
        let t_delta_z = if dir.z != 0.0 {
            (1.0 / dir.z).abs()
        } else {
            f32::INFINITY
        };

        let mut t_max_x = if dir.x > 0.0 {
            ((x as f32 + 1.0) - origin.x) * t_delta_x
        } else {
            (origin.x - x as f32) * t_delta_x
        };
        let mut t_max_y = if dir.y > 0.0 {
            ((y as f32 + 1.0) - origin.y) * t_delta_y
        } else {
            (origin.y - y as f32) * t_delta_y
        };
        let mut t_max_z = if dir.z > 0.0 {
            ((z as f32 + 1.0) - origin.z) * t_delta_z
        } else {
            (origin.z - z as f32) * t_delta_z
        };

        let mut current_distance = 0.0;
        let mut normal = glam::IVec3::ZERO;

        while current_distance <= max_distance {
            if chunks.is_solid_at(x, y, z) {
                return Some((glam::IVec3::new(x, y, z), normal));
            }

            if t_max_x < t_max_y {
                if t_max_x < t_max_z {
                    current_distance = t_max_x;
                    x += step_x;
                    t_max_x += t_delta_x;
                    normal = glam::IVec3::new(-step_x, 0, 0);
                } else {
                    current_distance = t_max_z;
                    z += step_z;
                    t_max_z += t_delta_z;
                    normal = glam::IVec3::new(0, 0, -step_z);
                }
            } else {
                if t_max_y < t_max_z {
                    current_distance = t_max_y;
                    y += step_y;
                    t_max_y += t_delta_y;
                    normal = glam::IVec3::new(0, -step_y, 0);
                } else {
                    current_distance = t_max_z;
                    z += step_z;
                    t_max_z += t_delta_z;
                    normal = glam::IVec3::new(0, 0, -step_z);
                }
            }
        }

        None
    }

    pub fn update_physics(&mut self, chunks: &crate::chunks::Chunks, dt: Duration) {
        if !chunks.is_chunk_loaded(self.position.x as i32, self.position.z as i32) {
            self.velocity = Vec3::ZERO;
            return;
        }

        let dt = dt.as_secs_f32();

        // Smoothly decay step offset
        if self.step_offset.abs() > 0.001 {
            self.step_offset *= f32::exp(-15.0 * dt);
        } else {
            self.step_offset = 0.0;
        }

        let radius = 0.3;
        let height_below = PLAYER_HEIGHT;
        let height_above = 0.2;

        // apply gravity
        self.velocity.y -= GRAVITY * dt;

        let d = self.velocity * dt;

        // Y axis
        self.position.y += d.y;
        if check_collision(self.position, radius, height_below, height_above, chunks) {
            if d.y < 0.0 {
                self.position.y = (self.position.y - height_below).floor() + 1.001 + height_below;
            } else if d.y > 0.0 {
                self.position.y = (self.position.y + height_above).floor() - 0.001 - height_above;
            }
            self.velocity.y = 0.0;
        }

        // X axis
        self.position.x += d.x;
        if check_collision(self.position, radius, height_below, height_above, chunks) {
            let mut stepped = false;
            let mut test_pos = self.position;
            for _ in 1..=10 {
                test_pos.y += 0.1;
                if !check_collision(test_pos, radius, height_below, height_above, chunks) {
                    let diff = test_pos.y - self.position.y;
                    self.position = test_pos;
                    self.step_offset -= diff;
                    stepped = true;
                    break;
                }
            }
            if !stepped {
                self.position.x -= d.x;
                self.velocity.x = 0.0;
            }
        }

        // Z axis
        self.position.z += d.z;
        if check_collision(self.position, radius, height_below, height_above, chunks) {
            let mut stepped = false;
            let mut test_pos = self.position;
            for _ in 1..=10 {
                test_pos.y += 0.1;
                if !check_collision(test_pos, radius, height_below, height_above, chunks) {
                    let diff = test_pos.y - self.position.y;
                    self.position = test_pos;
                    self.step_offset -= diff;
                    stepped = true;
                    break;
                }
            }
            if !stepped {
                self.position.z -= d.z;
                self.velocity.z = 0.0;
            }
        }
    }
}

fn check_collision(
    pos: Vec3,
    radius: f32,
    h_below: f32,
    h_above: f32,
    chunks: &crate::chunks::Chunks,
) -> bool {
    let min_x = (pos.x - radius).floor() as i32;
    let max_x = (pos.x + radius).floor() as i32;
    let min_y = (pos.y - h_below).floor() as i32;
    let max_y = (pos.y + h_above).floor() as i32;
    let min_z = (pos.z - radius).floor() as i32;
    let max_z = (pos.z + radius).floor() as i32;

    for x in min_x..=max_x {
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                if chunks.is_solid_at(x, y, z) {
                    return true;
                }
            }
        }
    }
    false
}

pub struct Controller {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,

    // These are velocities.
    amount_up: f32,
    amount_down: f32,

    is_jumping: bool,
    is_sprinting: bool,

    rotate_horiz: f32,
    rotate_vert: f32,

    scroll: f32,

    speed: f32,
    sensitivity: f32,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,

            // These are velocities.
            amount_up: 0.0,
            amount_down: 0.0,

            is_jumping: false,
            is_sprinting: false,

            rotate_horiz: 0.0,
            rotate_vert: 0.0,
            scroll: 0.0,

            speed: SPEED,
            sensitivity: SENSITIVITY,
        }
    }

    pub fn process_keyboard(
        &mut self,
        state: ElementState,
        _scancode: u32,
        keycode: KeyCode,
    ) -> bool {
        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };

        match keycode {
            //13 | 126 => {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.amount_forward = amount;
                true
            }
            //0 | 123 => {
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.amount_left = amount;
                true
            }
            //1 | 125 => {
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.amount_backward = amount;
                true
            }
            //2 | 124 => {
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.amount_right = amount;
                true
            }
            KeyCode::Space => {
                if !self.is_jumping && state == ElementState::Pressed {
                    self.amount_up = JUMP_VELOCITY;
                    self.is_jumping = true;
                }
                true
            }
            KeyCode::ShiftLeft | KeyCode::ShiftRight => {
                self.is_sprinting = state == ElementState::Pressed;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horiz = mouse_dx as f32;
        self.rotate_vert = -mouse_dy as f32;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = -match delta {
            MouseScrollDelta::LineDelta(_, scroll) => -scroll * 100.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => -(*scroll as f32),
        };
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();

        // walking around
        let (yaw_sin, yaw_cos) = camera.yaw.sin_cos();
        let forward = Vec3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vec3::new(-yaw_sin, 0.0, yaw_cos).normalize();

        let move_dir = forward * (self.amount_forward - self.amount_backward)
            + right * (self.amount_right - self.amount_left);
        let current_speed = if self.is_sprinting {
            self.speed * 2.0
        } else {
            self.speed
        };

        camera.velocity.x = move_dir.x * current_speed;
        camera.velocity.z = move_dir.z * current_speed;

        // zoom
        let (pitch_sin, pitch_cos) = camera.pitch.sin_cos();
        let scrollward = Vec3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        camera.position += scrollward * self.scroll * self.speed * self.sensitivity * dt;
        self.scroll = 0.0;

        // up and down
        if self.amount_up > 0.0 {
            camera.velocity.y = self.amount_up;
            self.amount_up = 0.0;
        }
        camera.position.y -= self.amount_down * self.speed * dt;

        if camera.velocity.y == 0.0 {
            self.is_jumping = false;
        }

        // rotate
        camera.yaw += self.rotate_horiz * self.sensitivity * dt;
        camera.pitch += self.rotate_vert * self.sensitivity * dt;

        self.rotate_horiz = 0.0;
        self.rotate_vert = 0.0;

        // limit pitch
        camera.pitch = camera.pitch.clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);
    }
}
