use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldConfig {
    pub seed: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub chunk_load_radius: i32,
    pub log_level: String,
    pub fov: f32,
    pub active_world: String,
    #[serde(default)]
    pub worlds: HashMap<String, WorldConfig>,
}

impl Default for Config {
    fn default() -> Self {
        let mut worlds = HashMap::new();
        worlds.insert(
            "funky_town".to_string(),
            WorldConfig { seed: None },
        );

        Self {
            chunk_load_radius: 3,
            log_level: "warn".to_string(),
            fov: 75.0,
            active_world: "funky_town".to_string(),
            worlds,
        }
    }
}

impl Config {
    pub fn load_or_create() -> Self {
        let config_path = "config.toml";
        let mut config = if let Ok(contents) = fs::read_to_string(config_path) {
            if let Ok(c) = toml::from_str::<Config>(&contents) {
                c
            } else {
                eprintln!("Failed to parse config.toml. Using defaults.");
                Config::default()
            }
        } else {
            Config::default()
        };

        let mut needs_save = false;
        
        // Ensure the active world exists in the map
        let world_config = config.worlds.entry(config.active_world.clone()).or_insert_with(|| {
            needs_save = true;
            WorldConfig { seed: None }
        });

        if world_config.seed.is_none() {
            let mut rng = rand::thread_rng();
            world_config.seed = Some(rng.gen::<u32>());
            needs_save = true;
        }

        if needs_save {
            if let Ok(toml_string) = toml::to_string(&config) {
                if let Err(e) = fs::write(config_path, toml_string) {
                    eprintln!("Failed to write config.toml: {}", e);
                }
            }
        }

        config
    }
}
