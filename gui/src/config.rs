use iced::Theme;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: Theme::KanagawaDragon,
            rounded: true,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ConfigFile {
    theme: String,
    #[serde(default = "default_true")]
    rounded: bool,
}

fn default_true() -> bool {
    true
}

impl From<&Config> for ConfigFile {
    fn from(c: &Config) -> Self {
        Self {
            theme: c.theme.to_string(),
            rounded: c.rounded,
        }
    }
}

impl From<ConfigFile> for Config {
    fn from(f: ConfigFile) -> Self {
        Self {
            theme: theme_from_str(&f.theme),
            rounded: f.rounded,
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
    dirs::config_dir().map(|d| d.join("msc").join("gui.toml"))
}

impl Config {
    pub fn load() -> Self {
        let Some(path) = config_path() else {
            return Self::default();
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => return Self::default(),
        };
        toml::from_str::<ConfigFile>(&text)
            .map(Into::into)
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = config_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("msc: could not create config dir: {e}");
                return;
            }
        }
        match toml::to_string_pretty(&ConfigFile::from(self)) {
            Ok(text) => {
                let _ = std::fs::write(&path, text);
            }
            Err(e) => eprintln!("msc: failed to serialize config: {e}"),
        }
    }
}
