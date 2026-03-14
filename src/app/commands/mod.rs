//! Command handlers for the CLI.

mod cache;
mod config;
mod doctor;
mod env;
mod info;
mod list;
mod run;
mod serve;
mod validate;

pub use cache::handle_cache;
pub use config::handle_config;
pub use doctor::handle_doctor;
pub use env::handle_env;
pub use info::handle_info;
pub use list::handle_ls;
pub use run::{handle_run, run_task_for_mcp};
pub use serve::handle_serve;
pub use validate::handle_validate;
