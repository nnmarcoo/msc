use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt::{self, Display},
    fs, io,
    path::PathBuf,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub root: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self { root: None }
    }
}

impl Config {
    fn path() -> Result<PathBuf, ConfigError> {
        let proj_dirs =
            directories::ProjectDirs::from("", "", "msc").ok_or(ConfigError::DirectoryNotFound)?;

        let config_dir = proj_dirs.config_dir();
        Ok(config_dir.join("config.toml"))
    }

    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let config_path = Self::path()?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let toml_string = toml::to_string_pretty(self)?;
        fs::write(&config_path, toml_string)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    Toml(toml::de::Error),
    TomlSerialize(toml::ser::Error),
    DirectoryNotFound,
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "IO error: {}", e),
            ConfigError::Toml(e) => write!(f, "TOML parse error: {}", e),
            ConfigError::TomlSerialize(e) => write!(f, "TOML serialize error: {}", e),
            ConfigError::DirectoryNotFound => write!(f, "Could not find config directory"),
        }
    }
}

impl Error for ConfigError {}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> Self {
        ConfigError::Io(err)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> Self {
        ConfigError::Toml(err)
    }
}

impl From<toml::ser::Error> for ConfigError {
    fn from(err: toml::ser::Error) -> Self {
        ConfigError::TomlSerialize(err)
    }
}
