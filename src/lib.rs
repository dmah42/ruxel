mod block;
mod camera;
mod instance;
mod light;
mod render_state;
mod scene;
mod terrain;
mod texture;
mod vertex;

use std::time::{Duration, Instant};

use render_state::RenderState;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
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
}

impl Ruxel {
    pub async fn new() -> Self {
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

        let state = RenderState::new(&window).await;
        Self {
            event_loop: Some(event_loop),
            window,
            state,
            camera_controller: camera::Controller::new(4.0, 0.4),
            mouse_pressed: false,
        }
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
            } => self
                .camera_controller
                .process_keyboard(*state, *scancode, *keycode),
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.mouse_pressed = *state == ElementState::Pressed;
                true
            }
            _ => false,
        }
    }

    fn update(&mut self, dt: Duration) {
        // TODO: update light.
        self.camera_controller
            .update_camera(self.state.camera(), dt);
        self.state.update(dt);
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
                    let dt = now - last_render_time;
                    last_render_time = now;
                    self.update(dt);
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
                    if self.mouse_pressed {
                        self.camera_controller.process_mouse(delta.0, delta.1)
                    }
                }
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == self.window.id() && !self.input(event) => match event {
                    #[cfg(not(target_arch = "wasm32"))]
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
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
