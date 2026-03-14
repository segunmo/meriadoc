//! Command dispatch - routes CLI commands to their handlers.

use crate::app::App;
use crate::app::commands::{
    handle_cache, handle_config, handle_doctor, handle_env, handle_info, handle_ls, handle_run,
    handle_serve, handle_validate,
};
use crate::cli::{Cli, Commands, RunKind};
use crate::core::validation::MeriadocError;

/// Dispatch a CLI command to its handler.
pub fn dispatch(cli: Cli, mut app: App) -> Result<(), MeriadocError> {
    let json = cli.json;

    match cli.command {
        Commands::Config { command } => handle_config(command, &mut app),

        Commands::Ls { target } => handle_ls(target, &app, json),

        Commands::Run {
            kind,
            name,
            options,
        } => handle_run(kind, name, options, &mut app),

        // Shortcut commands: meriadoc task/job/shell <name>
        Commands::Task { name, options } => handle_run(RunKind::Task, name, options, &mut app),
        Commands::Job { name, options } => handle_run(RunKind::Job, name, options, &mut app),
        Commands::Shell { name, options } => handle_run(RunKind::Shell, name, options, &mut app),

        Commands::Doctor => handle_doctor(&app),

        Commands::Cache { command } => handle_cache(command, &mut app),

        Commands::Info { target, name } => handle_info(target, name, &app, json),

        Commands::Validate { target } => handle_validate(target, &app),

        Commands::Env { command } => handle_env(command, &app, json),

        Commands::Serve => handle_serve(app),

        Commands::Server { port } => {
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                MeriadocError::Execution(format!("Failed to create async runtime: {}", e))
            })?;
            rt.block_on(crate::http::server::run_server(app, port))
        }
    }
}
