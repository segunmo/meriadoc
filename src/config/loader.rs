use crate::config::spec::MeriadocConfig;
use crate::core::validation::MeriadocError;
use serde_yaml;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ConfigLoader;

impl ConfigLoader {
    pub fn load(path: Option<PathBuf>) -> Result<MeriadocConfig, MeriadocError> {
        let path: PathBuf = path.unwrap_or(Self::resolve_config_path()?);

        if !path.exists() {
            let config: MeriadocConfig = MeriadocConfig::default();
            Self::save_to_path(&config, &path)?;
            return Ok(config);
        }

        let contents: String = fs::read_to_string(&path)?;
        Ok(serde_yaml::from_str(&contents)?)
    }

    pub fn save(config: &MeriadocConfig) -> Result<(), MeriadocError> {
        let path = Self::resolve_config_path()?;
        Self::save_to_path(config, &path)
    }

    pub fn resolve_config_path() -> Result<PathBuf, MeriadocError> {
        if let Ok(path) = std::env::var("MERIADOC_CONFIG") {
            return Ok(PathBuf::from(path));
        }

        let base: PathBuf = dirs::config_dir()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not resolve config directory",
                )
            })?
            .join("meriadoc");

        Ok(base.join("config.yaml"))
    }

    fn save_to_path(config: &MeriadocConfig, path: &Path) -> Result<(), MeriadocError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let serialized: String = serde_yaml::to_string(config)?;
        fs::write(path, serialized)?;
        Ok(())
    }
}
