use std::{
    fs::{read_to_string, write},
    io::Result,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub audio_directory: String,
    pub volume: f32,
}

impl Config {
    const CONFIG_PATH: &'static str = "config.json";

    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        write(Self::CONFIG_PATH, json)?;
        Ok(())
    }

    pub fn load() -> Result<Self> {
        let content = read_to_string(Self::CONFIG_PATH)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn get() -> Self {
        match Config::load() {
            Ok(config) => config,
            Err(_) => {
                let default_config = Config {
                    audio_directory: String::from("audio"),
                    volume: 1.0,
                };
                if let Err(e) = default_config.save() {
                    eprintln!("Failed to save default config: {}", e);
                }
                default_config
            }
        }
    }
}
