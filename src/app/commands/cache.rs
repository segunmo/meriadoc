//! Cache command handler.

use crate::app::App;
use crate::cli::CacheCommand;
use crate::core::validation::MeriadocError;

pub fn handle_cache(command: CacheCommand, app: &mut App) -> Result<(), MeriadocError> {
    match command {
        CacheCommand::Ls => {
            if app.cache.is_empty() {
                println!("Cache is empty.");
            } else {
                println!("Cached validations ({} entries):", app.cache.len());
                for (path, entry) in app.cache.list() {
                    let status = if entry.is_valid { "valid" } else { "invalid" };
                    println!("  {} [{}]", path.display(), status);
                }
            }
        }
        CacheCommand::Clear => {
            app.cache.clear();
            app.cache.save(&app.config.cache.dir)?;
            println!("Cache cleared.");
        }
    }
    Ok(())
}
