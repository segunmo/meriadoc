use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::core::execution::env::ResolvedEnv;
use crate::core::execution::interpolate::Interpolator;
use crate::core::validation::MeriadocError;

/// Result of executing a single command
#[derive(Debug)]
pub struct CommandResult {
    pub exit_code: i32,
    pub success: bool,
    /// Captured stdout (empty if capture_output is false)
    pub stdout: String,
    /// Captured stderr (empty if capture_output is false)
    pub stderr: String,
}

/// Workdir resolution mode
pub enum WorkdirMode {
    /// Default to spec file directory (for tasks/jobs)
    SpecFileDir,
    /// Default to current working directory (for shells)
    CurrentDir,
}

/// Options for command execution
#[derive(Debug, Clone, Default)]
pub struct ExecutionOptions {
    /// Print commands before execution
    pub verbose: bool,
    /// Timeout for each command (None = no timeout)
    pub timeout: Option<Duration>,
    /// Capture stdout/stderr instead of inheriting (for MCP)
    pub capture_output: bool,
    /// Names of environment variables that contain secrets (for masking in verbose output)
    pub secrets: HashSet<String>,
}

/// Mask secret values in a string by replacing them with "***"
fn mask_secrets(text: &str, env: &ResolvedEnv, secrets: &HashSet<String>) -> String {
    let mut masked = text.to_string();
    for secret_name in secrets {
        if let Some(value) = env.get(secret_name) {
            // Only mask non-empty values
            if !value.is_empty() {
                masked = masked.replace(value, "***");
            }
        }
    }
    masked
}

pub struct CommandRunner;

impl CommandRunner {
    /// Resolve working directory based on mode and optional override
    pub fn resolve_workdir(
        workdir_override: Option<&str>,
        project_root: &Path,
        spec_file_dir: &Path,
        mode: WorkdirMode,
    ) -> Result<PathBuf, MeriadocError> {
        let base = match mode {
            WorkdirMode::SpecFileDir => spec_file_dir.to_path_buf(),
            WorkdirMode::CurrentDir => std::env::current_dir()?,
        };

        match workdir_override {
            Some(override_path) => {
                // Workdir is relative to project root
                let resolved = project_root.join(override_path);
                if resolved.exists() && resolved.is_dir() {
                    Ok(resolved)
                } else {
                    Err(MeriadocError::Execution(format!(
                        "workdir does not exist: {}",
                        resolved.display()
                    )))
                }
            }
            None => Ok(base),
        }
    }

    /// Execute a single command with variable interpolation
    #[cfg(test)]
    pub fn run_command(
        cmd: &str,
        workdir: &Path,
        env: &ResolvedEnv,
    ) -> Result<CommandResult, MeriadocError> {
        Self::run_command_with_options(cmd, workdir, env, &ExecutionOptions::default())
    }

    /// Execute a single command with options (verbose, timeout, capture_output)
    pub fn run_command_with_options(
        cmd: &str,
        workdir: &Path,
        env: &ResolvedEnv,
        options: &ExecutionOptions,
    ) -> Result<CommandResult, MeriadocError> {
        // Interpolate variables in the command
        let interpolated_cmd = Interpolator::interpolate(cmd, env);

        // Verbose: print command before execution (with secrets masked)
        if options.verbose {
            let display_cmd = mask_secrets(&interpolated_cmd, env, &options.secrets);
            eprintln!("+ {}", display_cmd);
        }

        // Choose stdio mode based on capture setting
        let (stdout_mode, stderr_mode) = if options.capture_output {
            (Stdio::piped(), Stdio::piped())
        } else {
            (Stdio::inherit(), Stdio::inherit())
        };

        let child = Command::new("sh")
            .arg("-c")
            .arg(&interpolated_cmd)
            .current_dir(workdir)
            .envs(env.iter())
            .stdin(Stdio::inherit())
            .stdout(stdout_mode)
            .stderr(stderr_mode)
            .spawn()
            .map_err(|e| {
                MeriadocError::Execution(format!(
                    "failed to spawn command '{}': {}",
                    interpolated_cmd, e
                ))
            })?;

        // When capturing output, use wait_with_output (no timeout support)
        if options.capture_output {
            let output = child.wait_with_output().map_err(|e| {
                MeriadocError::Execution(format!("failed to wait for command: {}", e))
            })?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            // Apply truncation limit (100KB per stream)
            let stdout = truncate_output(stdout, 100 * 1024);
            let stderr = truncate_output(stderr, 100 * 1024);

            let exit_code = output.status.code().unwrap_or(128);
            return Ok(CommandResult {
                exit_code,
                success: output.status.success(),
                stdout,
                stderr,
            });
        }

        // Non-capture path: support timeout
        let mut child = child;
        let status = match options.timeout {
            Some(timeout_duration) => {
                // Wait with timeout using a background thread
                let (tx, rx) = mpsc::channel();

                // Spawn a timer thread
                let timeout_duration_clone = timeout_duration;
                thread::spawn(move || {
                    thread::sleep(timeout_duration_clone);
                    let _ = tx.send(());
                });

                // Poll the child process
                loop {
                    match child.try_wait() {
                        Ok(Some(status)) => break Ok(status),
                        Ok(None) => {
                            // Check if timeout expired
                            if rx.try_recv().is_ok() {
                                // Kill the process
                                let _ = child.kill();
                                let _ = child.wait(); // Reap the zombie
                                return Err(MeriadocError::Execution(format!(
                                    "command timed out after {:?}: {}",
                                    timeout_duration, interpolated_cmd
                                )));
                            }
                            // Small sleep to avoid busy-waiting
                            thread::sleep(Duration::from_millis(50));
                        }
                        Err(e) => {
                            break Err(MeriadocError::Execution(format!(
                                "failed to wait for command: {}",
                                e
                            )));
                        }
                    }
                }
            }
            None => child.wait().map_err(|e| {
                MeriadocError::Execution(format!("failed to wait for command: {}", e))
            }),
        }?;

        let exit_code = status.code().unwrap_or(128); // 128+ typically means killed by signal

        Ok(CommandResult {
            exit_code,
            success: status.success(),
            stdout: String::new(),
            stderr: String::new(),
        })
    }

    /// Execute multiple commands sequentially, stopping on first failure
    #[cfg(test)]
    pub fn run_commands(
        cmds: &[String],
        workdir: &Path,
        env: &ResolvedEnv,
    ) -> Result<Vec<CommandResult>, MeriadocError> {
        Self::run_commands_with_options(cmds, workdir, env, &ExecutionOptions::default())
    }

    /// Execute multiple commands with options (verbose, timeout)
    pub fn run_commands_with_options(
        cmds: &[String],
        workdir: &Path,
        env: &ResolvedEnv,
        options: &ExecutionOptions,
    ) -> Result<Vec<CommandResult>, MeriadocError> {
        let mut results = Vec::new();

        for cmd in cmds {
            let result = Self::run_command_with_options(cmd, workdir, env, options)?;
            let success = result.success;
            results.push(result);

            if !success {
                break; // Stop on first failure
            }
        }

        Ok(results)
    }

    /// Interpolate a command without executing (for dry-run display)
    pub fn interpolate_command(cmd: &str, env: &ResolvedEnv) -> String {
        Interpolator::interpolate(cmd, env)
    }
}

/// Truncate output to max_bytes, adding a truncation message if needed.
fn truncate_output(output: String, max_bytes: usize) -> String {
    if output.len() <= max_bytes {
        output
    } else {
        // Find a valid UTF-8 boundary near max_bytes
        let truncated = &output[..max_bytes];
        // Find the last valid char boundary
        let boundary = truncated
            .char_indices()
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);
        format!(
            "{}... [truncated, {} bytes total]",
            &output[..boundary],
            output.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_command_success() {
        let env = ResolvedEnv::new();
        let workdir = std::env::current_dir().unwrap();
        let result = CommandRunner::run_command("true", &workdir, &env).unwrap();
        assert!(result.success);
        assert_eq!(result.exit_code, 0);
    }

    #[test]
    fn test_run_command_failure() {
        let env = ResolvedEnv::new();
        let workdir = std::env::current_dir().unwrap();
        let result = CommandRunner::run_command("false", &workdir, &env).unwrap();
        assert!(!result.success);
        assert_eq!(result.exit_code, 1);
    }

    #[test]
    fn test_run_command_with_timeout_success() {
        let env = ResolvedEnv::new();
        let workdir = std::env::current_dir().unwrap();
        let options = ExecutionOptions {
            verbose: false,
            timeout: Some(Duration::from_secs(5)),
            capture_output: false,
            ..Default::default()
        };
        let result =
            CommandRunner::run_command_with_options("echo hello", &workdir, &env, &options)
                .unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_run_command_with_timeout_expires() {
        let env = ResolvedEnv::new();
        let workdir = std::env::current_dir().unwrap();
        let options = ExecutionOptions {
            verbose: false,
            timeout: Some(Duration::from_millis(100)),
            capture_output: false,
            ..Default::default()
        };
        let result = CommandRunner::run_command_with_options("sleep 10", &workdir, &env, &options);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }

    #[test]
    fn test_run_commands_stops_on_failure() {
        let env = ResolvedEnv::new();
        let workdir = std::env::current_dir().unwrap();
        let cmds = vec![
            "true".to_string(),
            "false".to_string(),
            "echo should-not-run".to_string(),
        ];
        let results = CommandRunner::run_commands(&cmds, &workdir, &env).unwrap();
        assert_eq!(results.len(), 2); // Stops after 'false'
        assert!(results[0].success);
        assert!(!results[1].success);
    }

    #[test]
    fn test_capture_stdout() {
        let env = ResolvedEnv::new();
        let workdir = std::env::current_dir().unwrap();
        let options = ExecutionOptions {
            verbose: false,
            timeout: None,
            capture_output: true,
            ..Default::default()
        };
        let result =
            CommandRunner::run_command_with_options("echo hello", &workdir, &env, &options)
                .unwrap();
        assert!(result.success);
        assert_eq!(result.stdout.trim(), "hello");
        assert!(result.stderr.is_empty());
    }

    #[test]
    fn test_capture_stderr() {
        let env = ResolvedEnv::new();
        let workdir = std::env::current_dir().unwrap();
        let options = ExecutionOptions {
            verbose: false,
            timeout: None,
            capture_output: true,
            ..Default::default()
        };
        let result =
            CommandRunner::run_command_with_options("echo error >&2", &workdir, &env, &options)
                .unwrap();
        assert!(result.success);
        assert!(result.stdout.is_empty());
        assert_eq!(result.stderr.trim(), "error");
    }

    #[test]
    fn test_capture_both_streams() {
        let env = ResolvedEnv::new();
        let workdir = std::env::current_dir().unwrap();
        let options = ExecutionOptions {
            verbose: false,
            timeout: None,
            capture_output: true,
            ..Default::default()
        };
        let result = CommandRunner::run_command_with_options(
            "echo out; echo err >&2",
            &workdir,
            &env,
            &options,
        )
        .unwrap();
        assert!(result.success);
        assert_eq!(result.stdout.trim(), "out");
        assert_eq!(result.stderr.trim(), "err");
    }

    #[test]
    fn test_no_capture_returns_empty() {
        let env = ResolvedEnv::new();
        let workdir = std::env::current_dir().unwrap();
        let options = ExecutionOptions {
            verbose: false,
            timeout: None,
            capture_output: false,
            ..Default::default()
        };
        let result =
            CommandRunner::run_command_with_options("echo hello", &workdir, &env, &options)
                .unwrap();
        assert!(result.success);
        assert!(result.stdout.is_empty());
        assert!(result.stderr.is_empty());
    }

    #[test]
    fn test_truncate_output_short() {
        let output = "hello".to_string();
        let truncated = super::truncate_output(output.clone(), 100);
        assert_eq!(truncated, output);
    }

    #[test]
    fn test_truncate_output_long() {
        let output = "a".repeat(200);
        let truncated = super::truncate_output(output.clone(), 100);
        assert!(truncated.len() < output.len());
        assert!(truncated.contains("[truncated, 200 bytes total]"));
    }
}
