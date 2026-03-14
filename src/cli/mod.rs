//! Command-line interface definitions.

pub mod options;
pub mod parsers;
pub mod prompt;

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

pub use options::{InteractiveMode, RunOptions};
pub use prompt::{CliPrompter, EditableVar, EnvPrompter};

/// Meriadoc — task & job runner
#[derive(Parser, Debug)]
#[command(name = "meriadoc")]
#[command(version)]
#[command(about = "Run, validate and inspect Meriadoc projects", long_about = None)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    /// Path to config file (overrides default location)
    #[arg(long = "config", global = true)]
    pub config: Option<PathBuf>,

    /// Output in JSON format (for programmatic consumption)
    #[arg(long = "json", global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run a task, job or shell
    Run {
        #[arg(value_enum)]
        kind: RunKind,
        name: String,
        #[command(flatten)]
        options: RunOptions,
    },

    /// Run a task (shortcut for 'run task')
    #[command(visible_alias = "t")]
    Task {
        name: String,
        #[command(flatten)]
        options: RunOptions,
    },

    /// Run a job (shortcut for 'run job')
    #[command(visible_alias = "j")]
    Job {
        name: String,
        #[command(flatten)]
        options: RunOptions,
    },

    /// Start a shell (shortcut for 'run shell')
    #[command(visible_alias = "s")]
    Shell {
        name: String,
        #[command(flatten)]
        options: RunOptions,
    },

    /// List entities
    Ls {
        #[arg(value_enum)]
        target: Option<ListTarget>,
    },

    /// Show detailed info
    Info {
        #[arg(value_enum)]
        target: InfoTarget,
        name: String,
    },

    /// Validate specs
    Validate {
        #[command(subcommand)]
        target: Option<ValidateTarget>,
    },

    /// Manage config
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },

    /// Cache operations
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
    },

    /// Environment variable operations
    Env {
        #[command(subcommand)]
        command: EnvCommand,
    },

    /// Diagnose common problems
    Doctor,

    /// Start MCP server for AI agent integration (stdio)
    Serve,

    /// Start HTTP server with web UI
    Server {
        /// Port to listen on
        #[arg(short, long, default_value = "8420")]
        port: u16,
    },
}

#[derive(ValueEnum, Debug, Clone)]
pub enum RunKind {
    Task,
    Job,
    Shell,
}

#[derive(ValueEnum, Debug, Clone)]
pub enum ListTarget {
    Projects,
    Tasks,
    Jobs,
    Shells,
}

#[derive(ValueEnum, Debug, Clone)]
pub enum InfoTarget {
    Project,
    Task,
    Job,
    Shell,
}

#[derive(Subcommand, Debug)]
pub enum ValidateTarget {
    Project { name: String },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    Ls,
    Add { path: PathBuf },
    Rm { path: PathBuf },
}

#[derive(Subcommand, Debug)]
pub enum CacheCommand {
    Ls,
    Clear,
}

#[derive(Subcommand, Debug)]
pub enum EnvCommand {
    /// Show environment variables for a task, job, or shell
    Show {
        #[arg(value_enum)]
        target: EnvTarget,
        name: String,
    },

    /// List saved environment files
    Ls,

    /// Generate a template env file for a task, job, or shell
    Init {
        #[arg(value_enum)]
        target: EnvTarget,
        name: String,
    },

    /// Delete a saved environment file
    Rm {
        /// Project name
        project: String,
        /// Entity name (task, job, or shell)
        entity: String,
    },
}

#[derive(ValueEnum, Debug, Clone)]
pub enum EnvTarget {
    Task,
    Job,
    Shell,
}
