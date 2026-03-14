use std::path::PathBuf;

use crate::app::App;
use crate::cli::ConfigCommand;
use crate::config::spec::DiscoveryRoot;
use crate::config::{ConfigLoader, MeriadocConfig};
use crate::core::validation::MeriadocError;
use crate::repo::discovery::ProjectDiscovery;

pub fn handle_config(command: ConfigCommand, app: &mut App) -> Result<(), MeriadocError> {
    match command {
        ConfigCommand::Add { path } => {
            ConfigActions::add_path(&mut app.config, path)?;
            ConfigLoader::save(&app.config)?;
        }
        ConfigCommand::Rm { path } => {
            ConfigActions::disable_path(&mut app.config, path)?;
            ConfigLoader::save(&app.config)?;
        }
        ConfigCommand::Ls => {
            for root in &app.config.discovery.roots {
                println!(
                    "{} [{}]",
                    root.path.display(),
                    if root.enabled { "enabled" } else { "disabled" }
                );
            }
        }
    }
    Ok(())
}

struct ConfigActions;

impl ConfigActions {
    pub fn add_path(config: &mut MeriadocConfig, path: PathBuf) -> Result<(), MeriadocError> {
        let root = normalize_path(&path)?;

        let projects = ProjectDiscovery::discover(
            &root,
            &config.discovery.spec_files,
            config.discovery.max_depth,
        );

        for project in projects {
            match config
                .discovery
                .roots
                .iter_mut()
                .find(|r| r.path == project.root)
            {
                Some(existing) => {
                    existing.enabled = true;
                }
                None => {
                    config.discovery.roots.push(DiscoveryRoot {
                        path: project.root,
                        name: None,
                        enabled: true,
                    });
                }
            }
        }

        Ok(())
    }

    pub fn disable_path(config: &mut MeriadocConfig, path: PathBuf) -> Result<(), MeriadocError> {
        let normalized = normalize_path(&path)?;

        match config
            .discovery
            .roots
            .iter_mut()
            .find(|r| r.path == normalized)
        {
            Some(root) => root.enabled = false,
            None => {
                println!("Discovery root not found: {}", normalized.display());
            }
        }

        Ok(())
    }
}

fn normalize_path(path: &PathBuf) -> Result<PathBuf, MeriadocError> {
    let absolute = if path.is_absolute() {
        path.clone()
    } else {
        std::env::current_dir()?.join(path)
    };

    Ok(std::fs::canonicalize(absolute)?)
}
