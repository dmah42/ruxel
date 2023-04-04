use bytemuck::{Pod, Zeroable};
use winit::event::{ElementState, KeyboardInput, WindowEvent};

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct Uniform {
    view_proj: [f32; 16],
}

impl Uniform {
    pub fn new() -> Self {
        Self {
            view_proj: *glam::Mat4::IDENTITY.as_ref(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = *camera.view_projection_matrix().as_ref();
    }
}

pub struct Camera {
    pub eye: glam::Vec3,
    pub target: glam::Vec3,
    pub up: glam::Vec3,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    fn view_projection_matrix(&self) -> glam::Mat4 {
        let view = glam::Mat4::look_at_rh(self.eye, self.target, self.up);
        let proj = glam::Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar);
        proj * view
    }
}

pub struct Controller {
    speed: f32,
    is_forward: bool,
    is_backward: bool,
    is_left: bool,
    is_right: bool,
}

impl Controller {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward: false,
            is_backward: false,
            is_left: false,
            is_right: false,
        }
    }

    pub fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input: KeyboardInput {
                    state, scancode, ..
                },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match scancode {
                    // W | Up
                    13 | 126 => {
                        self.is_forward = is_pressed;
                        true
                    }
                    // A | Left
                    0 | 123 => {
                        self.is_left = is_pressed;
                        true
                    }
                    // S | Down
                    1 | 125 => {
                        self.is_backward = is_pressed;
                        true
                    }
                    // D | Right
                    2 | 124 => {
                        self.is_right = is_pressed;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn update_camera(&self, camera: &mut Camera) {
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.length();

        if self.is_forward && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed;
        }
        if self.is_backward {
            camera.eye -= forward_norm * self.speed;
        }

        let right = forward_norm.cross(camera.up);

        let forward = camera.target - camera.eye;
        let forward_mag = forward.length();

        if self.is_right {
            camera.eye = camera.target - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.is_left {
            camera.eye = camera.target - (forward - right * self.speed).normalize() * forward_mag;
        }
    }
}
