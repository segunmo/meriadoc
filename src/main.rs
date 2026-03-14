mod app;
mod cli;
mod config;
mod core;
mod http;
mod mcp;
mod repo;

use clap::Parser;

use crate::app::App;
use crate::cli::Cli;
use crate::config::ConfigLoader;
use crate::core::validation::MeriadocError;

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), MeriadocError> {
    let cli = Cli::parse();
    let config = ConfigLoader::load(cli.config.clone())?;
    let app = App::new(config)?;

    app::dispatch::dispatch(cli, app)
}
