use crate::config::spec::MeriadocConfig;
use crate::core::validation::MeriadocError;
use serde_yaml;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ConfigLoader;

impl ConfigLoader {
    pub fn load(path: Option<PathBuf>) -> Result<MeriadocConfig, MeriadocError> {
        let path: PathBuf = path.unwrap_or(Self::resolve_config_path()?);

        let mut config: MeriadocConfig = if !path.exists() {
            let config = MeriadocConfig::default();
            Self::save_to_path(&config, &path)?;
            config
        } else {
            let contents: String = fs::read_to_string(&path)?;
            serde_yaml::from_str(&contents)?
        };

        // Normalize cache dir: if relative or empty, replace with the absolute default.
        if !config.cache.dir.is_absolute() {
            config.cache.dir = Self::default_cache_base()?;
        }

        Ok(config)
    }

    /// Returns the default absolute cache base directory: <config_dir>/meriadoc/cache
    pub fn default_cache_base() -> Result<PathBuf, MeriadocError> {
        let base = dirs::config_dir()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not resolve config directory",
                )
            })?
            .join("meriadoc")
            .join("cache");
        Ok(base)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_config(dir: &TempDir, content: &str) -> PathBuf {
        let path = dir.path().join("config.yaml");
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_relative_cache_dir_is_replaced_with_absolute() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "cache:\n  enabled: true\n  dir: .meriadoc/cache\n");
        let config = ConfigLoader::load(Some(path)).unwrap();
        assert!(
            config.cache.dir.is_absolute(),
            "relative cache dir should be replaced with absolute: {:?}",
            config.cache.dir
        );
    }

    #[test]
    fn test_empty_cache_dir_sentinel_is_replaced_with_absolute() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "cache:\n  enabled: true\n  dir: ''\n");
        let config = ConfigLoader::load(Some(path)).unwrap();
        assert!(config.cache.dir.is_absolute());
    }

    #[test]
    fn test_absolute_cache_dir_is_preserved() {
        let dir = TempDir::new().unwrap();
        let abs = dir.path().join("my-cache");
        let path = write_config(
            &dir,
            &format!("cache:\n  enabled: true\n  dir: {}\n", abs.display()),
        );
        let config = ConfigLoader::load(Some(path)).unwrap();
        assert_eq!(config.cache.dir, abs);
    }

    #[test]
    fn test_default_config_has_absolute_cache_dir() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.yaml");
        // Path doesn't exist — loader creates default and returns it
        let config = ConfigLoader::load(Some(path)).unwrap();
        assert!(config.cache.dir.is_absolute());
    }
}
