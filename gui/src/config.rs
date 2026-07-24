use std::path::PathBuf;

use iced::Theme;
use serde::{Deserialize, Serialize};

pub const VOLUME_DEFAULT: f32 = 0.5;

pub const ALL_THEMES: &[Theme] = &[
    Theme::Light,
    Theme::Dark,
    Theme::Dracula,
    Theme::Nord,
    Theme::SolarizedLight,
    Theme::SolarizedDark,
    Theme::GruvboxLight,
    Theme::GruvboxDark,
    Theme::CatppuccinLatte,
    Theme::CatppuccinFrappe,
    Theme::CatppuccinMacchiato,
    Theme::CatppuccinMocha,
    Theme::TokyoNight,
    Theme::TokyoNightStorm,
    Theme::TokyoNightLight,
    Theme::KanagawaWave,
    Theme::KanagawaDragon,
    Theme::KanagawaLotus,
    Theme::Moonfly,
    Theme::Nightfly,
    Theme::Oxocarbon,
    Theme::Ferra,
];

#[derive(Debug, Clone)]
pub struct Config {
    pub theme: Theme,
    pub rounded: bool,
    pub volume: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: Theme::KanagawaDragon,
            rounded: true,
            volume: VOLUME_DEFAULT,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ConfigFile {
    #[serde(default)]
    theme: String,
    #[serde(default = "default_true")]
    rounded: bool,
    #[serde(default = "default_volume")]
    volume: f32,
}

fn default_true() -> bool {
    true
}

fn default_volume() -> f32 {
    VOLUME_DEFAULT
}

impl From<&Config> for ConfigFile {
    fn from(c: &Config) -> Self {
        Self {
            theme: c.theme.to_string(),
            rounded: c.rounded,
            volume: c.volume,
        }
    }
}

impl From<ConfigFile> for Config {
    fn from(f: ConfigFile) -> Self {
        Self {
            theme: theme_from_str(&f.theme),
            rounded: f.rounded,
            volume: f.volume.clamp(0.0, 1.0),
        }
    }
}

fn theme_from_str(s: &str) -> Theme {
    ALL_THEMES
        .iter()
        .find(|t| t.to_string() == s)
        .cloned()
        .unwrap_or(Theme::KanagawaDragon)
}

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("verse").join("config.toml"))
}

impl Config {
    pub fn load() -> Self {
        let Some(path) = config_path() else {
            return Self::default();
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        toml::from_str::<ConfigFile>(&text)
            .map(Into::into)
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = config_path() else {
            return;
        };
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            eprintln!("verse: could not create config dir: {e}");
            return;
        }
        match toml::to_string_pretty(&ConfigFile::from(self)) {
            Ok(text) => {
                if let Err(e) = std::fs::write(&path, text) {
                    eprintln!("verse: failed to write config: {e}");
                }
            }
            Err(e) => eprintln!("verse: failed to serialize config: {e}"),
        }
    }
}
