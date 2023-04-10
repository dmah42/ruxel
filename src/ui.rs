use std::time::Duration;

use glam::{IVec2, Vec3};
use wgpu::{CommandBuffer, Device, Queue, SurfaceConfiguration, TextureView};
use wgpu_text::{
    font::FontArc,
    section::{Section, Text},
    BrushBuilder, TextBrush,
};

pub struct Ui {
    brush: TextBrush,
    player_position: String,
    block_position: String,
    chunk_position: String,
}

impl Ui {
    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
        let font_data = include_bytes!("../fonts/Stacked pixel.ttf").to_vec();
        let font = FontArc::try_from_vec(font_data).expect("unable to load font");
        let brush = BrushBuilder::using_font(font).build(&device, &config);

        Self {
            brush,
            player_position: String::from(""),
            block_position: String::from(""),
            chunk_position: String::from(""),
        }
    }

    pub fn update(
        &mut self,
        player_position: &Vec3,
        block_position: &IVec2,
        chunk_position: &IVec2,
        _dt: Duration,
    ) {
        self.player_position = format!(
            "player: {:.2} {:.2} {:.2}",
            player_position.x, player_position.y, player_position.z
        );
        self.block_position = format!("block: {block_position}");
        self.chunk_position = format!("chunk: {chunk_position}");
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, queue: &Queue) {
        self.brush
            .resize_view(new_size.width as f32, new_size.height as f32, queue);
    }

    pub fn render(&mut self, device: &Device, queue: &Queue, view: &TextureView) -> CommandBuffer {
        self.brush.queue(
            Section::default()
                .add_text(Text::new(&self.player_position).with_scale(30.0))
                .with_screen_position((20.0, 20.0)),
        );
        self.brush.queue(
            Section::default()
                .add_text(Text::new(&self.block_position).with_scale(30.0))
                .with_screen_position((20.0, 55.0)),
        );
        self.brush.queue(
            Section::default()
                .add_text(Text::new(&self.chunk_position).with_scale(30.0))
                .with_screen_position((20.0, 90.0)),
        );
        self.brush
            .process_queued(device, queue)
            .expect("failed to process queue");
        self.brush.draw(device, view)
    }
}
