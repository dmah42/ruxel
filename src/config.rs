use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub chunk_load_radius: i32,
    pub seed: Option<u32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            chunk_load_radius: 3,
            seed: None,
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
        if config.seed.is_none() {
            let mut rng = rand::thread_rng();
            config.seed = Some(rng.gen::<u32>());
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
