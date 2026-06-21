mod block;
mod camera;
mod chunks;
pub mod config;
mod light;
mod mesh;
mod render_state;
mod scene;
mod sky;
mod terrain;
mod texture;
mod ui;
mod vertex;

use std::time::{Duration, Instant};

use render_state::RenderState;
use std::sync::Arc;
use ui::Ui;
use rand::Rng;
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowBuilder},
};

const REACH_DISTANCE: f32 = 6.0;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use winit::dpi::PhysicalSize;

pub struct Ruxel {
    event_loop: Option<EventLoop<()>>,
    window: Arc<Window>,
    state: RenderState<'static>,
    scene: scene::Scene,
    camera: camera::Camera,
    camera_controller: camera::Controller,
    mouse_pressed: bool,
    mouse_grabbed: bool,
    received_mouse_motion: bool,
    last_cursor_pos: Option<winit::dpi::PhysicalPosition<f64>>,
    selected_block_type: block::Type,
    ui: Ui,
    config: config::Config,
    last_save_time: Instant,
}

impl Ruxel {
    pub async fn new() -> Result<Self, winit::error::EventLoopError> {
        let config = config::Config::load_or_create();

        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                std::panic::set_hook(Box::new(console_error_panic_hook::hook));
                console_log::init_with_level(log::Level::Warn).expect("failed to init console_log");
            } else {
                env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(&config.log_level)).init();
            }
        }

        let seed = config.worlds.get(&config.active_world).unwrap().seed.expect("Seed should always be present after load_or_create");
        log::info!("Loaded config: {:?}", config);
        let event_loop = EventLoop::new()?;
        let window = WindowBuilder::new()
            .with_min_inner_size(PhysicalSize::new(1024, 768))
            .with_inner_size(PhysicalSize::new(1920, 1080))
            .with_title("ruxel")
            .build(&event_loop)
            .expect("failed to build window");
        let window = Arc::new(window);

        #[cfg(target_arch = "wasm32")]
        {
            use winit::dpi::PhysicalSize;
            window.set_inner_size(PhysicalSize::new(450, 400));

            use winit::platform::web::WindowExtWebSys;
            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| {
                    let dst = doc.get_element_by_id("wasm-example")?;
                    let canvas = web_sys::Element::from(window.canvas());
                    dst.append_child(&canvas).ok()?;
                    Some(())
                })
                .expect("failed to add canvas to document body");
        }

        let scene = scene::Scene::new(seed, config.clone());

        let (player_x, player_y, player_z, yaw, pitch) = if let Some(cam) = config.worlds.get(&config.active_world).and_then(|w| w.camera.as_ref()) {
            (cam.position[0], cam.position[1], cam.position[2], cam.yaw, cam.pitch)
        } else {
            let mut rng = rand::thread_rng();
            let mut px = rng.gen_range(2000.0..4000.0);
            let mut pz = rng.gen_range(2000.0..4000.0);

            while scene.chunks().height_at(&glam::Vec3::new(px, 0.0, pz)) <= 32.0 {
                px = rng.gen_range(2000.0..4000.0);
                pz = rng.gen_range(2000.0..4000.0);
            }

            let spawn_height = scene.chunks().height_at(&glam::Vec3::new(px, 0.0, pz));
            (px, spawn_height + 5.0, pz, 0.0, 0.0)
        };

        let size = window.inner_size();
        let aspect = size.width as f32 / size.height as f32;

        let camera = camera::Camera::new(
            glam::Vec3::new(player_x, player_y, player_z),
            yaw,
            pitch,
            aspect,
            config.fov,
            config.chunk_load_radius,
        );

        let state = RenderState::new(config.clone(), window.clone()).await;
        let ui = Ui::new();

        Ok(Self {
            event_loop: Some(event_loop),
            window,
            state,
            scene,
            camera,
            camera_controller: camera::Controller::new(),
            mouse_pressed: false,
            mouse_grabbed: false,
            received_mouse_motion: false,
            last_cursor_pos: None,
            selected_block_type: block::Type::Grass,
            ui,
            config,
            last_save_time: Instant::now(),
        })
    }

    fn grab_mouse(&mut self) {
        if let Err(e) = self.window.set_cursor_grab(CursorGrabMode::Locked) {
            log::warn!(
                "Failed to grab mouse with Locked mode: {:?}. Falling back to Confined.",
                e
            );
            let _ = self.window.set_cursor_grab(CursorGrabMode::Confined);
        }
        self.window.set_cursor_visible(false);
        self.mouse_grabbed = true;
    }

    fn ungrab_mouse(&mut self) {
        let _ = self.window.set_cursor_grab(CursorGrabMode::None);
        self.window.set_cursor_visible(true);
        self.mouse_grabbed = false;
    }

    fn interact(&mut self, place: bool, block_type: crate::block::Type) {
        if let Some((hit_pos, normal)) = self.camera.raycast(self.scene.chunks(), REACH_DISTANCE) {
            if place {
                let p = hit_pos + normal;
                self.scene.chunks().set_block(p.x, p.y, p.z, block_type);
            } else {
                self.scene.chunks().set_block(
                    hit_pos.x,
                    hit_pos.y,
                    hit_pos.z,
                    crate::block::Type::Inactive,
                );
            }
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(keycode),
                        state,
                        ..
                    },
                ..
            } => {
                if *state == ElementState::Pressed {
                    match keycode {
                        KeyCode::Digit1 => self.selected_block_type = block::Type::Grass,
                        KeyCode::Digit2 => self.selected_block_type = block::Type::Sand,
                        KeyCode::Digit3 => self.selected_block_type = block::Type::Rock,
                        KeyCode::Digit4 => self.selected_block_type = block::Type::Ice,
                        KeyCode::Digit5 => self.selected_block_type = block::Type::Water,
                        _ => {}
                    }
                }
                self.camera_controller.process_keyboard(*state, 0, *keycode)
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                let pressed = *state == ElementState::Pressed;
                if pressed && !self.mouse_pressed {
                    if !self.mouse_grabbed {
                        self.grab_mouse();
                    } else {
                        self.interact(false, self.selected_block_type);
                    }
                }
                self.mouse_pressed = pressed;
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state: ElementState::Pressed,
                ..
            } => {
                if !self.mouse_grabbed {
                    self.grab_mouse();
                } else {
                    self.interact(true, self.selected_block_type);
                }
                true
            }
            _ => false,
        }
    }

    fn update(&mut self, dt: Duration, selected_block_type: block::Type) {
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera.update_physics(self.scene.chunks(), dt);
        self.scene.update(dt, &self.camera);
        
        let selected_block = self.camera.raycast(self.scene.chunks(), REACH_DISTANCE).map(|(pos, _)| pos);
        
        let player_pos = self.camera.position();
        let point = [player_pos.x as f64 / 384.0, player_pos.z as f64 / 384.0];
        let blend_str = self.scene.chunks().terrain().biome_blend_string(point);

        self.ui.update(
            &player_pos,
            self.scene.chunks().block_position(),
            self.scene.chunks().chunk_position(),
            selected_block_type,
            blend_str,
            dt,
        );
        
        self.state.update(dt, &self.camera, &self.scene, selected_block);

        if self.last_save_time.elapsed() > Duration::from_secs(5) {
            self.save_config();
        }
    }

    fn save_config(&mut self) {
        if let Some(world_config) = self.config.worlds.get_mut(&self.config.active_world) {
            self.camera.save_state(world_config);
        }
        self.config.save();
        self.last_save_time = Instant::now();
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
    pub async fn run(mut self) {
        let event_loop = self
            .event_loop
            .take()
            .expect("unexpected lack of event loop");

        let mut last_render_time = Instant::now();

        let _ = event_loop.run(move |event, control_flow| {
            control_flow.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent {
                    window_id,
                    event: WindowEvent::RedrawRequested,
                } if window_id == self.window.id() => {
                    let now = Instant::now();
                    let mut dt = now - last_render_time;
                    if dt > Duration::from_millis(100) {
                        dt = Duration::from_millis(100);
                    }
                    last_render_time = now;
                    let block_type = self.selected_block_type;
                    self.update(dt, block_type);
                    match self.state.render(&self.ui) {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => self.state.resize(self.state.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            println!("out of memory");
                            self.save_config();
                            control_flow.exit();
                        }
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
                Event::AboutToWait => {
                    self.window.request_redraw();
                }
                Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion { delta },
                    ..
                } => {
                    self.received_mouse_motion = true;
                    if self.mouse_grabbed {
                        let scale = self.window.scale_factor();
                        self.camera_controller.process_mouse(delta.0 * scale, delta.1 * scale);
                    }
                }
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == self.window.id() && !self.input(event) => match event {
                    #[cfg(not(target_arch = "wasm32"))]
                    WindowEvent::CloseRequested => {
                        self.save_config();
                        control_flow.exit();
                    }
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::Escape),
                                ..
                            },
                        ..
                    } => {
                        if self.mouse_grabbed {
                            self.ungrab_mouse();
                        } else {
                            self.save_config();
                            control_flow.exit();
                        }
                    }
                    WindowEvent::Focused(focused) => {
                        if !focused {
                            self.mouse_pressed = false;
                            if self.mouse_grabbed {
                                self.ungrab_mouse();
                            }
                        }
                    }
                    WindowEvent::Resized(physical_size) => {
                        if physical_size.width > 0 && physical_size.height > 0 {
                            self.camera.projection.resize(physical_size.width, physical_size.height);
                        }
                        self.state.resize(*physical_size);
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        if self.mouse_grabbed {
                            if !self.received_mouse_motion {
                                if let Some(last_pos) = self.last_cursor_pos {
                                    let dx = position.x - last_pos.x;
                                    let dy = position.y - last_pos.y;
                                    self.camera_controller.process_mouse(dx, dy);
                                }

                                self.last_cursor_pos = Some(*position);
                            } else {
                                self.last_cursor_pos = Some(*position);
                            }
                        } else {
                            self.last_cursor_pos = Some(*position);
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        });
    }
}
