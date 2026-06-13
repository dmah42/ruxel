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
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{CursorGrabMode, Window, WindowBuilder},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use winit::dpi::PhysicalSize;

pub struct Ruxel {
    event_loop: Option<EventLoop<()>>,
    window: Window,
    state: RenderState,
    camera_controller: camera::Controller,
    mouse_pressed: bool,
    mouse_grabbed: bool,
    selected_block_type: block::Type,
}

impl Ruxel {
    pub async fn new(seed: u32) -> Self {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                std::panic::set_hook(Box::new(console_error_panic_hook::hook));
                console_log::init_with_level(log::Level::Warn).expect("failed to init console_log");
            } else {
                env_logger::init();
            }
        }
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_min_inner_size(PhysicalSize::new(1024, 768))
            .with_inner_size(PhysicalSize::new(1920, 1080))
            .with_title("ruxel")
            .build(&event_loop)
            .expect("failed to build window");

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

        let state = RenderState::new(seed, &window).await;

        Self {
            event_loop: Some(event_loop),
            window,
            state,
            camera_controller: camera::Controller::new(4.0, 0.4),
            mouse_pressed: false,
            mouse_grabbed: false,
            selected_block_type: block::Type::Grass,
        }
    }

    fn grab_mouse(&mut self) {
        if self.window.set_cursor_grab(CursorGrabMode::Locked).is_err() {
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
                input:
                    KeyboardInput {
                        virtual_keycode: Some(keycode),
                        scancode,
                        state,
                        ..
                    },
                ..
            } => {
                if *state == ElementState::Pressed {
                    match keycode {
                        VirtualKeyCode::Key1 => self.selected_block_type = block::Type::Grass,
                        VirtualKeyCode::Key2 => self.selected_block_type = block::Type::Sand,
                        VirtualKeyCode::Key3 => self.selected_block_type = block::Type::Rock,
                        VirtualKeyCode::Key4 => self.selected_block_type = block::Type::Ice,
                        VirtualKeyCode::Key5 => self.selected_block_type = block::Type::Water,
                        _ => {}
                    }
                }
                self.camera_controller
                    .process_keyboard(*state, *scancode, *keycode)
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

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::RedrawRequested(window_id) if window_id == self.window.id() => {
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
                            *control_flow = ControlFlow::Exit;
                        }
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
                Event::MainEventsCleared => {
                    self.window.request_redraw();
                }
                Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion { delta },
                    ..
                } => {
                    if self.mouse_grabbed {
                        self.camera_controller.process_mouse(delta.0, delta.1);
                    }
                }
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == self.window.id() && !self.input(event) => match event {
                    #[cfg(not(target_arch = "wasm32"))]
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => {
                        if self.mouse_grabbed {
                            self.ungrab_mouse();
                        } else {
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                    WindowEvent::Resized(physical_size) => {
                        self.state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        self.state.resize(**new_inner_size);
                    }
                    _ => {}
                },
                _ => {}
            }
        });
    }
}
