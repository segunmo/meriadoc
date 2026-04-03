//! Cache command handler.

use std::collections::HashMap;

use crate::app::App;
use crate::cli::CacheCommand;
use crate::core::resolver::EntityResolver;
use crate::core::validation::MeriadocError;

pub fn handle_cache(command: CacheCommand, app: &mut App) -> Result<(), MeriadocError> {
    match command {
        CacheCommand::Ls => {
            let total: usize = app.caches.values().map(|c| c.len()).sum();
            if total == 0 {
                println!("Cache is empty.");
            } else {
                // Detect duplicate project names so we can disambiguate in output.
                let mut name_count: HashMap<&str, usize> = HashMap::new();
                for project in &app.projects {
                    *name_count
                        .entry(EntityResolver::project_name(project))
                        .or_insert(0) += 1;
                }

                println!("Cached validations ({} entries):", total);
                for project in &app.projects {
                    let project_name = EntityResolver::project_name(project);
                    if let Some(cache) = app.caches.get(&project.root)
                        && !cache.is_empty()
                    {
                        // Show full path when two discovered projects share the same name.
                        let label = if name_count.get(project_name).copied().unwrap_or(0) > 1 {
                            format!("{} ({})", project_name, project.root.display())
                        } else {
                            project_name.to_string()
                        };
                        println!("  [{}] ({} entries)", label, cache.len());
                        for (path, entry) in cache.list() {
                            let status = if entry.is_valid { "valid" } else { "invalid" };
                            println!("    {} [{}]", path.display(), status);
                        }
                    }
                }
            }
        }
        CacheCommand::Clear => {
            // Wipe the entire cache base directory so orphaned dirs from renamed
            // or removed projects are also cleaned up.
            let base = &app.config.cache.dir;
            if base.exists() {
                std::fs::remove_dir_all(base)?;
            }
            app.caches.values_mut().for_each(|c| c.clear());
            println!("Cache cleared.");
        }
    }
    Ok(())
}
