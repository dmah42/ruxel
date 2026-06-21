use std::time::Duration;

use crate::{block, console};
use glam::{IVec2, Vec3};

pub struct Ui {
    pub(crate) player_position: String,
    pub(crate) block_position: String,
    pub(crate) chunk_position: String,
    pub(crate) target: String,
    pub(crate) fps_str: String,
    pub(crate) selected_block: String,
    pub(crate) biome: String,
    pub(crate) is_console_open: bool,
    pub(crate) console_text: String,
    fps: u32,
    total_time: Duration,
}

impl Default for Ui {
    fn default() -> Self {
        Self::new()
    }
}

pub struct UiContext<'a> {
    pub player_position: &'a Vec3,
    pub block_position: &'a IVec2,
    pub chunk_position: &'a IVec2,
    pub selected_block_type: block::Type,
    pub blend_str: String,
    pub dt: Duration,
    pub console: &'a crate::console::Console,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            player_position: String::from(""),
            block_position: String::from(""),
            chunk_position: String::from(""),
            target: String::from("+"),
            fps: 0,
            total_time: Duration::new(0, 0),
            fps_str: String::from("FPS: 0"),
            selected_block: String::from(""),
            biome: String::from(""),
            is_console_open: false,
            console_text: String::from(""),
        }
    }

    pub fn update(&mut self, ctx: UiContext) {
        self.player_position = format!(
            "player: {:.2} {:.2} {:.2}",
            ctx.player_position.x, ctx.player_position.y, ctx.player_position.z
        );
        self.block_position = format!("block: {} {}", ctx.block_position.x, ctx.block_position.y);
        self.chunk_position = format!("chunk: {} {}", ctx.chunk_position.x, ctx.chunk_position.y);
        self.selected_block = format!("selected: {:?}", ctx.selected_block_type);
        self.biome = format!("biome: {}", ctx.blend_str);

        self.fps += 1;
        self.total_time += ctx.dt;
        if self.total_time.as_secs_f32() > 1.0 {
            self.fps_str = format!("FPS: {}", self.fps);
            self.fps = 0;
            self.total_time = Duration::new(0, 0);
        }

        self.is_console_open = ctx.console.is_open();
        if self.is_console_open {
            let mut text = ctx.console.history().join("\n");
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(console::PROMPT_GLYPH);
            text.push_str(ctx.console.input());
            self.console_text = text;
        } else {
            self.console_text.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::console::Console;

    #[test]
    fn test_ui_update_fps() {
        let mut ui = Ui::new();
        let dt = Duration::from_millis(600);
        let console = Console::new();

        ui.update(UiContext {
            player_position: &Vec3::ZERO,
            block_position: &IVec2::ZERO,
            chunk_position: &IVec2::ZERO,
            selected_block_type: block::Type::Grass,
            blend_str: "TestBiome".to_string(),
            dt,
            console: &console,
        });
        assert_eq!(ui.fps, 1);
        assert_eq!(ui.fps_str, "FPS: 0"); // hasn't updated string yet

        ui.update(UiContext {
            player_position: &Vec3::ZERO,
            block_position: &IVec2::ZERO,
            chunk_position: &IVec2::ZERO,
            selected_block_type: block::Type::Grass,
            blend_str: "TestBiome".to_string(),
            dt,
            console: &console,
        });
        assert_eq!(ui.fps, 0);
        assert_eq!(ui.fps_str, "FPS: 2"); // string updated
    }
}
