use serde::{Deserialize, Serialize};
use std::{borrow::Cow, path::PathBuf};

/// Root configuration file for Meriadoc (~/.config/meriadoc/config.yaml)
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct MeriadocConfig {
    /// Paths where projects can be discovered
    #[serde(default)]
    pub discovery: DiscoveryConfig,

    /// Cache configuration
    #[serde(default)]
    pub cache: CacheConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// List of root directories to search for meriadoc.yaml
    pub roots: Vec<DiscoveryRoot>,

    /// Maximum directory depth when searching for specs
    pub max_depth: usize,

    /// Whether discovery should validate specs immediately
    pub validate_on_discovery: bool,

    /// Names of specfiles accepted
    pub spec_files: Vec<Cow<'static, str>>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            max_depth: 3,
            validate_on_discovery: true,
            spec_files: vec![
                Cow::Borrowed("meriadoc.yaml"),
                Cow::Borrowed("meriadoc.yml"),
                Cow::Borrowed("merry.yaml"),
                Cow::Borrowed("merry.yml"),
            ],
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveryRoot {
    /// Root path for discovery
    pub path: PathBuf,

    /// Optional human-friendly name
    pub name: Option<String>,

    /// Whether this root is currently enabled
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable or disable cache entirely
    pub enabled: bool,

    /// Directory where cached specs and metadata are stored
    pub dir: PathBuf,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dir: PathBuf::from(".meriadoc/cache"),
        }
    }
}
