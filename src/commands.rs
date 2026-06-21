use crate::{camera::Camera, scene::Scene, console::TimeOfDay};
use glam::Vec3;

pub(crate) fn execute_teleport(camera: &mut Camera, x: f32, y: f32, z: f32) -> String {
    camera.set_position(Vec3::new(x, y, z));
    format!("Teleported to {:.2}, {:.2}, {:.2}", x, y, z)
}

pub(crate) fn execute_time(scene: &mut Scene, time: Option<TimeOfDay>) -> String {
    match time {
        Some(t) => {
            let t_rads = t.to_radians();
            scene.set_time(t_rads);
            format!("Set time to {}", t)
        }
        None => {
            let t = scene.time();
            let name = if t < 0.0 {
                "night"
            } else if t < std::f32::consts::FRAC_PI_3 {
                "morning"
            } else if t < 2.0 * std::f32::consts::FRAC_PI_3 {
                "day"
            } else {
                "evening"
            };
            format!("Current time: {}", name)
        }
    }
}

pub(crate) fn execute_find_biome(scene: &mut Scene, camera: &Camera, biome_name: &str) -> String {
    let terrain = scene.chunks().terrain();
    let start_pos = camera.position();

    let target_biome = match crate::terrain::Biome::from_str(biome_name) {
        Some(b) => b,
        None => return format!("Unknown biome: '{}'", biome_name),
    };

    match terrain.find_closest_pure_biome(start_pos.x as f64, start_pos.z as f64, target_biome) {
        Some(point) => {
            let px = point[0];
            let pz = point[1];
            let height = scene
                .chunks()
                .height_at(&glam::Vec3::new(px as f32, 0.0, pz as f32));
            let dist =
                ((px - start_pos.x as f64).powi(2) + (pz - start_pos.z as f64).powi(2)).sqrt();
            format!(
                "Found 100% {} at {:.0}, {:.0}, {:.0} ({:.0} blocks away)",
                biome_name, px, height, pz, dist
            )
        }
        None => format!("Biome '{}' not found within search radius.", biome_name),
    }
}

pub(crate) fn execute_help(command: Option<String>) -> String {
    match command.as_deref() {
        None => "Available commands: help, tp/teleport, time, fb/find_biome".to_string(),
        Some("help") => "help [command] - Lists all available commands, or provides help for a specific command.".to_string(),
        Some("tp") | Some("teleport") => "teleport <x> <y> <z> - Teleports the player to the specified coordinates.".to_string(),
        Some("time") => "time [time_of_day] - Sets the time (morning, day, evening, night). If empty, prints current time.".to_string(),
        Some("fb") | Some("find_biome") => "find_biome <biome> - Finds the nearest chunk of the specified biome (e.g. desert, plains).".to_string(),
        Some(cmd) => format!("Unknown command for help: {}", cmd),
    }
}
