use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    sync::{OnceLock, RwLock},
};
use thiserror::Error;

static CONFIG: OnceLock<RwLock<Config>> = OnceLock::new();

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub(crate) root: Option<PathBuf>,
}

fn project_dirs() -> Result<directories::ProjectDirs, ConfigError> {
    directories::ProjectDirs::from("", "", "msc").ok_or(ConfigError::DirectoryNotFound)
}

impl Config {
    pub(crate) fn path() -> Result<PathBuf, ConfigError> {
        let proj_dirs = project_dirs()?;
        Ok(proj_dirs.config_dir().join("config.toml"))
    }

    pub(crate) fn database_path() -> Result<PathBuf, ConfigError> {
        let proj_dirs = project_dirs()?;
        Ok(proj_dirs.data_dir().join("library.db"))
    }

    pub fn init() -> Result<(), ConfigError> {
        let config = Self::load_from_disk()?;
        CONFIG
            .set(RwLock::new(config))
            .map_err(|_| ConfigError::AlreadyInitialized)?;
        Ok(())
    }

    pub fn get() -> &'static RwLock<Config> {
        CONFIG
            .get()
            .expect("Config not initialized. Call Config::init() first.")
    }

    pub fn root() -> Option<PathBuf> {
        Self::get().read().unwrap().root.clone()
    }

    pub fn set_root(path: PathBuf) -> Result<(), ConfigError> {
        let mut config = Self::get().write().unwrap();
        config.root = Some(path);
        config.save()?;
        Ok(())
    }

    pub fn save_current() -> Result<(), ConfigError> {
        let config = Self::get().read().unwrap();
        config.save()
    }

    fn load_from_disk() -> Result<Self, ConfigError> {
        let config_path = Self::path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub(crate) fn save(&self) -> Result<(), ConfigError> {
        let config_path = Self::path()?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let toml_string = toml::to_string_pretty(self)?;
        fs::write(&config_path, toml_string)?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),
    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("Could not find config directory")]
    DirectoryNotFound,
    #[error("Config already initialized")]
    AlreadyInitialized,
}
