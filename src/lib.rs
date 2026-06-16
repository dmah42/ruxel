mod block;
mod camera;
mod chunks;
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
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowBuilder},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use winit::dpi::PhysicalSize;

pub struct Ruxel {
    event_loop: Option<EventLoop<()>>,
    window: Arc<Window>,
    state: RenderState<'static>,
    camera_controller: camera::Controller,
    mouse_pressed: bool,
    mouse_grabbed: bool,
    received_mouse_motion: bool,
    last_cursor_pos: Option<winit::dpi::PhysicalPosition<f64>>,
    selected_block_type: block::Type,
}

impl Ruxel {
    pub async fn new(seed: u32) -> Result<Self, winit::error::EventLoopError> {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                std::panic::set_hook(Box::new(console_error_panic_hook::hook));
                console_log::init_with_level(log::Level::Warn).expect("failed to init console_log");
            } else {
                env_logger::init();
            }
        }
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

        let state = RenderState::new(seed, window.clone()).await;

        Ok(Self {
            event_loop: Some(event_loop),
            window,
            state,
            camera_controller: camera::Controller::new(4.0, 0.4),
            mouse_pressed: false,
            mouse_grabbed: false,
            received_mouse_motion: false,
            last_cursor_pos: None,
            selected_block_type: block::Type::Grass,
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
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
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
                        self.state.interact(false, self.selected_block_type);
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
                    self.state.interact(true, self.selected_block_type);
                }
                true
            }
            _ => false,
        }
    }

    fn update(&mut self, dt: Duration, selected_block_type: block::Type) {
        self.camera_controller
            .update_camera(self.state.camera(), dt);
        {
            // update player physics
            self.state.update_physics(dt);
        }
        self.state.update(dt, selected_block_type);
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
                    match self.state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => self.state.resize(self.state.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            println!("out of memory");
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
                    WindowEvent::CloseRequested => control_flow.exit(),
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
                        self.state.resize(*physical_size);
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        if self.mouse_grabbed {
                            if !self.received_mouse_motion {
                                if let Some(last_pos) = self.last_cursor_pos {
                                    let dx = position.x - last_pos.x;
                                    let dy = position.y - last_pos.y;
                                    println!("DEBUG EVENT: CursorMoved dx: {}, dy: {}", dx, dy);
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
