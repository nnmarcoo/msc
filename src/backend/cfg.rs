use std::{
    env::current_dir,
    fs::{read_to_string, write},
    io::Result,
};

use serde::{Deserialize, Serialize};

use super::playlist::Playlist;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub audio_directory: String,
    pub volume: f32,
    pub redraw: bool,
    pub redraw_time: f32,
    pub show_image: bool,
    pub playlists: Vec<Playlist>,
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
                let mut current_dir = current_dir().unwrap();
                current_dir.push("audio");

                let default_config = Config {
                    audio_directory: current_dir.to_string_lossy().to_string(),
                    volume: 1.0,
                    redraw: true,
                    redraw_time: 0.1,
                    show_image: true,
                    playlists: Vec::new(),
                };
                if let Err(e) = default_config.save() {
                    eprintln!("Failed to save default config: {}", e);
                }
                default_config
            }
        }
    }
}
