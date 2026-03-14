//! Shared types for run commands.

use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::Duration;

use clap::Args;

use super::parsers::parse_key_val;

/// Common options for running tasks, jobs, and shells.
#[derive(Args, Debug, Clone)]
pub struct RunOptions {
    /// Override or define environment variables (KEY=VALUE)
    #[arg(long = "env", value_parser = parse_key_val, number_of_values = 1)]
    pub env: Vec<(String, String)>,

    /// Load environment variables from a file (.env style)
    #[arg(long = "env-file")]
    pub env_file: Option<PathBuf>,

    /// Show what would be executed without running anything
    #[arg(long = "dry-run")]
    pub dry_run: bool,

    /// Never prompt for missing variables, fail immediately
    #[arg(short = 'n', long = "no-interactive")]
    pub no_interactive: bool,

    /// Always prompt for missing variables, even if not a TTY
    #[arg(short = 'i', long = "interactive")]
    pub interactive: bool,

    /// Prompt for ALL variables, showing current values for review/edit
    #[arg(long = "prompt-all")]
    pub prompt_all: bool,

    /// Print commands before execution
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Timeout for each command in seconds (0 = no timeout)
    #[arg(long = "timeout", value_parser = parse_timeout)]
    pub timeout: Option<Duration>,
}

/// Parse timeout from seconds string to Duration
fn parse_timeout(s: &str) -> Result<Duration, String> {
    let secs: u64 = s.parse().map_err(|_| format!("invalid timeout: {}", s))?;
    if secs == 0 {
        Err("timeout must be greater than 0".to_string())
    } else {
        Ok(Duration::from_secs(secs))
    }
}

/// Interactive mode configuration for prompting behavior.
#[derive(Debug, Clone, Copy)]
pub struct InteractiveMode {
    /// Never prompt, always fail on missing vars
    pub no_interactive: bool,
    /// Always prompt for missing vars, even if not a TTY
    pub interactive: bool,
    /// Prompt for ALL variables, showing current values
    pub prompt_all: bool,
}

impl InteractiveMode {
    /// Create from RunOptions
    pub fn from_options(options: &RunOptions) -> Self {
        Self {
            no_interactive: options.no_interactive,
            interactive: options.interactive,
            prompt_all: options.prompt_all,
        }
    }

    /// Should we prompt for missing variables?
    pub fn should_prompt(&self) -> bool {
        if self.no_interactive {
            return false;
        }
        if self.interactive || self.prompt_all {
            return true;
        }
        // Default: prompt only if TTY
        std::io::stdin().is_terminal()
    }

    /// Should we prompt for ALL variables (including already-set ones)?
    pub fn should_prompt_all(&self) -> bool {
        self.prompt_all && !self.no_interactive
    }
}
