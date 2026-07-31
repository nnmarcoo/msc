//! Persisted GUI settings (theme, volume, saved layouts). Split into an owned
//! `Config` and a serialized `ConfigFile` so on-disk changes stay forgiving.
//!
//! `muted` is kept beside `volume` rather than folded into it as a zero, so
//! unmuting knows what level to go back to across a restart.
//!
//! A short `layouts` list is padded back up to `PRESET_SLOTS` from the defaults
//! rather than left as saved, so a config written before the slots were fixed —
//! or hand-edited down — still answers every key bound to a slot. Padding takes
//! the defaults at the same position, which leaves what the user saved untouched
//! and only fills the tail they never had.

use std::path::PathBuf;

use iced::Theme;
use serde::{Deserialize, Serialize};

use crate::keybinds::{Keymap, KeymapFile};
use crate::layout::{Layout, PRESET_SLOTS, default_presets};

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
    pub muted: bool,
    pub keymap: Keymap,
    pub layouts: Vec<Layout>,
    pub active_layout: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: Theme::KanagawaDragon,
            rounded: true,
            volume: VOLUME_DEFAULT,
            muted: false,
            keymap: Keymap::default(),
            layouts: default_presets(),
            active_layout: 0,
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
    #[serde(default)]
    muted: bool,
    #[serde(default)]
    keybinds: KeymapFile,
    #[serde(default)]
    layouts: Vec<Layout>,
    #[serde(default)]
    active_layout: usize,
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
            muted: c.muted,
            keybinds: KeymapFile::from(&c.keymap),
            layouts: c.layouts.clone(),
            active_layout: c.active_layout,
        }
    }
}

impl From<ConfigFile> for Config {
    fn from(f: ConfigFile) -> Self {
        let layouts = fill_slots(f.layouts);
        let active_layout = f.active_layout.min(layouts.len() - 1);

        Self {
            theme: theme_from_str(&f.theme),
            rounded: f.rounded,
            volume: f.volume.clamp(0.0, verse_core::VOLUME_MAX),
            muted: f.muted,
            keymap: Keymap::from(f.keybinds),
            layouts,
            active_layout,
        }
    }
}

fn fill_slots(mut layouts: Vec<Layout>) -> Vec<Layout> {
    let defaults = default_presets();
    if layouts.is_empty() {
        return defaults;
    }
    layouts.truncate(PRESET_SLOTS);
    layouts.extend(defaults.into_iter().skip(layouts.len()));
    layouts
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
