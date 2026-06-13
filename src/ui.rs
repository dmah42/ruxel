use std::time::Duration;

use glam::{IVec2, Vec2, Vec3};
use wgpu::{Device, Queue, SurfaceConfiguration};
use wgpu_text::{
    glyph_brush::{ab_glyph::FontArc, Section, Text},
    BrushBuilder, TextBrush,
};

pub struct Ui {
    brush: TextBrush,
    player_position: String,
    block_position: String,
    chunk_position: String,
    target: String,
    center: Vec2,
    fps: u32,
    total_time: Duration,
    fps_str: String,
    selected_block: String,
}

impl Ui {
    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
        let font_data = include_bytes!("../fonts/Stacked pixel.ttf").to_vec();
        let font = FontArc::try_from_vec(font_data).expect("unable to load font");
        let brush = BrushBuilder::using_font(font).build(
            device,
            config.width,
            config.height,
            config.format,
        );

        Self {
            brush,
            player_position: String::from(""),
            block_position: String::from(""),
            chunk_position: String::from(""),
            target: String::from("+"),
            center: Vec2::new(config.width as f32 / 2.0, config.height as f32 / 2.0),
            fps: 0,
            total_time: Duration::new(0, 0),
            fps_str: String::from("FPS: 0"),
            selected_block: String::from(""),
        }
    }

    pub fn update(
        &mut self,
        player_position: &Vec3,
        block_position: &IVec2,
        chunk_position: &IVec2,
        selected_block_type: crate::block::Type,
        dt: Duration,
    ) {
        self.player_position = format!(
            "player: {:.2} {:.2} {:.2}",
            player_position.x, player_position.y, player_position.z
        );
        self.block_position = format!("block: {block_position}");
        self.chunk_position = format!("chunk: {chunk_position}");
        self.selected_block = format!("selected: {:?}", selected_block_type);

        self.fps += 1;
        self.total_time += dt;
        if self.total_time.as_secs_f32() > 1.0 {
            self.fps_str = format!("FPS: {}", self.fps);
            self.fps = 0;
            self.total_time = Duration::new(0, 0);
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, queue: &Queue) {
        self.center = Vec2::new(new_size.width as f32 / 2.0, new_size.height as f32 / 2.0);
        self.brush
            .resize_view(new_size.width as f32, new_size.height as f32, queue);
    }

    pub fn render<'pass>(
        &'pass mut self,
        device: &Device,
        queue: &Queue,
        rpass: &mut wgpu::RenderPass<'pass>,
    ) {
        self.brush
            .queue(
                device,
                queue,
                vec![
                    Section::default()
                        .add_text(Text::new(&self.player_position).with_scale(30.0))
                        .with_screen_position((20.0, 20.0)),
                    Section::default()
                        .add_text(Text::new(&self.block_position).with_scale(30.0))
                        .with_screen_position((20.0, 55.0)),
                    Section::default()
                        .add_text(Text::new(&self.chunk_position).with_scale(30.0))
                        .with_screen_position((20.0, 90.0)),
                    Section::default()
                        .add_text(Text::new(&self.fps_str).with_scale(30.0))
                        .with_screen_position((900.0, 20.0)),
                    Section::default()
                        .add_text(
                            Text::new(&self.target)
                                .with_scale(48.0)
                                .with_color([0.0, 0.0, 0.0, 0.7]),
                        )
                        .with_screen_position(self.center),
                    Section::default()
                        .add_text(Text::new(&self.selected_block).with_scale(30.0))
                        .with_screen_position((20.0, 125.0)),
                ],
            )
            .expect("failed to process UI queue");
        self.brush.draw(rpass)
    }
}
