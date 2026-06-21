use std::time::Duration;

use crate::block;
use glam::{IVec2, Vec3};

pub struct Ui {
    pub(crate) player_position: String,
    pub(crate) block_position: String,
    pub(crate) chunk_position: String,
    pub(crate) target: String,
    pub(crate) fps_str: String,
    pub(crate) selected_block: String,
    pub(crate) biome: String,
    fps: u32,
    total_time: Duration,
}

impl Default for Ui {
    fn default() -> Self {
        Self::new()
    }
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
        }
    }

    pub fn update(
        &mut self,
        player_position: &Vec3,
        block_position: &IVec2,
        chunk_position: &IVec2,
        selected_block_type: block::Type,
        blend_str: String,
        dt: Duration,
    ) {
        self.player_position = format!(
            "player: {:.2} {:.2} {:.2}",
            player_position.x, player_position.y, player_position.z
        );
        self.block_position = format!("block: {block_position}");
        self.chunk_position = format!("chunk: {chunk_position}");
        self.selected_block = format!("selected: {:?}", selected_block_type);
        self.biome = format!("biome: {}", blend_str);

        self.fps += 1;
        self.total_time += dt;
        if self.total_time.as_secs_f32() > 1.0 {
            self.fps_str = format!("FPS: {}", self.fps);
            self.fps = 0;
            self.total_time = Duration::new(0, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_update_fps() {
        let mut ui = Ui::new();
        let dt = Duration::from_millis(600);
        
        ui.update(&Vec3::ZERO, &IVec2::ZERO, &IVec2::ZERO, block::Type::Grass, "TestBiome".to_string(), dt);
        assert_eq!(ui.fps, 1);
        assert_eq!(ui.fps_str, "FPS: 0"); // hasn't updated string yet
        
        ui.update(&Vec3::ZERO, &IVec2::ZERO, &IVec2::ZERO, block::Type::Grass, "TestBiome".to_string(), dt);
        assert_eq!(ui.fps, 0);
        assert_eq!(ui.fps_str, "FPS: 2"); // string updated
    }
}
