use std::{f32::consts::FRAC_PI_2, time::Duration};

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseScrollDelta, VirtualKeyCode},
};

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.001;
const GRAVITY: f32 = 3.0;
const PLAYER_HEIGHT: f32 = 1.8;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Uniform {
    view_proj: [f32; 16],
}

impl Uniform {
    pub fn new() -> Self {
        Self {
            view_proj: *Mat4::IDENTITY.as_ref(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view_proj = *(projection.matrix() * camera.matrix()).as_ref();
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
    velocity_y: f32,
}

impl Camera {
    pub fn new(position: Vec3, yaw: f32, pitch: f32) -> Self {
        Self {
            position,
            yaw,
            pitch,
            velocity_y: 0.0,
        }
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    fn matrix(&self) -> Mat4 {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();

        Mat4::look_to_rh(
            self.position,
            Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
            Vec3::Y,
        )
    }

    pub fn update_physics(&mut self, height: f32, dt: Duration) {
        let dt = dt.as_secs_f32();
        let height = height + PLAYER_HEIGHT;
        // gravity
        if self.position.y > height {
            self.velocity_y -= GRAVITY * dt;
            self.position.y += self.velocity_y * dt;
        }
        // collision with ground
        if self.position.y < height {
            self.position.y = height;
            self.velocity_y = 0.0;
        }
    }
}

pub struct Controller {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,

    rotate_horiz: f32,
    rotate_vert: f32,

    scroll: f32,

    speed: f32,
    sensitivity: f32,
}

impl Controller {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,

            rotate_horiz: 0.0,
            rotate_vert: 0.0,
            scroll: 0.0,

            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(
        &mut self,
        state: ElementState,
        _scancode: u32,
        keycode: VirtualKeyCode,
    ) -> bool {
        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match keycode {
            //13 | 126 => {
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.amount_forward = amount;
                true
            }
            //0 | 123 => {
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.amount_left = amount;
                true
            }
            //1 | 125 => {
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.amount_backward = amount;
                true
            }
            //2 | 124 => {
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.amount_right = amount;
                true
            }
            //VirtualKeyCode::Space => {
            //    self.amount_up = amount;
            //    true
            //}
            //VirtualKeyCode::LShift => {
            //    self.amount_down = amount;
            //    true
            //}
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

        camera.position += forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
        camera.position += right * (self.amount_right - self.amount_left) * self.speed * dt;

        // zoom
        let (pitch_sin, pitch_cos) = camera.pitch.sin_cos();
        let scrollward = Vec3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        camera.position += scrollward * self.scroll * self.speed * self.sensitivity * dt;
        self.scroll = 0.0;

        // up and down
        // TODO: jump
        camera.position.y += (self.amount_up - self.amount_down) * self.speed * dt;

        // rotate
        camera.yaw += self.rotate_horiz * self.sensitivity * dt;
        camera.pitch += self.rotate_vert * self.sensitivity * dt;

        self.rotate_horiz = 0.0;
        self.rotate_vert = 0.0;

        // limit pitch
        if camera.pitch < -SAFE_FRAC_PI_2 {
            camera.pitch = -SAFE_FRAC_PI_2;
        } else if camera.pitch > SAFE_FRAC_PI_2 {
            camera.pitch = SAFE_FRAC_PI_2;
        }
    }
}
