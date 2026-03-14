use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

use crate::core::execution::env::ResolvedEnv;
use crate::core::validation::MeriadocError;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

pub struct InteractiveShell;

impl InteractiveShell {
    /// Start an interactive shell with the given environment
    /// This replaces the current process with the shell
    #[cfg(unix)]
    pub fn start(
        workdir: &Path,
        env: &ResolvedEnv,
        init_cmds: &[String],
        shell_name: &str,
        project_name: &str,
    ) -> Result<(), MeriadocError> {
        // Get user's preferred shell
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let shell_type = Self::detect_shell_type(&shell);

        // Build environment with Meriadoc markers
        let mut shell_env = env.clone();
        shell_env.insert("MERIADOC_SHELL".to_string(), shell_name.to_string());
        shell_env.insert("MERIADOC_PROJECT".to_string(), project_name.to_string());

        // Build init commands script
        let init_script = if init_cmds.is_empty() {
            String::new()
        } else {
            init_cmds.join(" && ") + " && "
        };

        // Create temporary rc file and exec the shell
        match shell_type {
            ShellType::Zsh => Self::exec_zsh(&shell, workdir, &shell_env, &init_script, shell_name),
            ShellType::Bash => {
                Self::exec_bash(&shell, workdir, &shell_env, &init_script, shell_name)
            }
            ShellType::Other => {
                Self::exec_generic(&shell, workdir, &shell_env, &init_script, shell_name)
            }
        }
    }

    /// Detect shell type from shell path
    fn detect_shell_type(shell_path: &str) -> ShellType {
        let path = Path::new(shell_path);
        let shell_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Check if the shell name contains the shell identifier
        // This handles cases like "zsh", "zsh-5.9", "/opt/homebrew/bin/zsh"
        if shell_name.contains("zsh") {
            ShellType::Zsh
        } else if shell_name.contains("bash") {
            ShellType::Bash
        } else {
            ShellType::Other
        }
    }

    #[cfg(unix)]
    fn exec_zsh(
        shell: &str,
        workdir: &Path,
        env: &ResolvedEnv,
        init_script: &str,
        shell_name: &str,
    ) -> Result<(), MeriadocError> {
        // Create a temp directory for ZDOTDIR
        // Using tempfile crate ensures cleanup on drop, but since we exec(),
        // we need to leak it intentionally (the OS will clean up /tmp eventually)
        let temp_dir = TempDir::new()
            .map_err(|e| MeriadocError::Execution(format!("failed to create temp dir: {}", e)))?;

        // Get user's home directory for sourcing original .zshrc
        let home = std::env::var("HOME").unwrap_or_default();

        // Create .zshrc that sources original and modifies prompt
        let zshrc_content = format!(
            r#"# Meriadoc shell wrapper
# Source original zshrc if it exists
if [[ -f "{home}/.zshrc" ]]; then
    source "{home}/.zshrc"
fi

# Run init commands
{init_script}true

# Modify prompt with green prefix
PROMPT="%F{{green}}({name})%f $PROMPT"
"#,
            home = home,
            init_script = init_script,
            name = shell_name
        );

        let zshrc_path = temp_dir.path().join(".zshrc");
        std::fs::write(&zshrc_path, zshrc_content)
            .map_err(|e| MeriadocError::Execution(format!("failed to write temp .zshrc: {}", e)))?;

        // Also create .zshenv to source the original
        let zshenv_content = format!(
            r#"# Meriadoc shell wrapper
if [[ -f "{home}/.zshenv" ]]; then
    source "{home}/.zshenv"
fi
"#,
            home = home
        );
        let zshenv_path = temp_dir.path().join(".zshenv");
        std::fs::write(&zshenv_path, zshenv_content).map_err(|e| {
            MeriadocError::Execution(format!("failed to write temp .zshenv: {}", e))
        })?;

        // Set ZDOTDIR and exec zsh
        let mut shell_env = env.clone();
        shell_env.insert(
            "ZDOTDIR".to_string(),
            temp_dir.path().to_string_lossy().to_string(),
        );

        // Keep the TempDir so it's not deleted before exec
        // The OS will clean /tmp periodically
        let _ = temp_dir.keep();

        let err = Command::new(shell)
            .arg("-i")
            .current_dir(workdir)
            .envs(shell_env.iter())
            .exec();

        Err(MeriadocError::Execution(format!(
            "failed to exec zsh: {}",
            err
        )))
    }

    #[cfg(unix)]
    fn exec_bash(
        shell: &str,
        workdir: &Path,
        env: &ResolvedEnv,
        init_script: &str,
        shell_name: &str,
    ) -> Result<(), MeriadocError> {
        // Create a temp bashrc file
        let temp_dir = TempDir::new()
            .map_err(|e| MeriadocError::Execution(format!("failed to create temp dir: {}", e)))?;

        // Get user's home directory for sourcing original .bashrc
        let home = std::env::var("HOME").unwrap_or_default();

        // Create bashrc that sources original and modifies prompt
        let bashrc_content = format!(
            r#"# Meriadoc shell wrapper
# Source original bashrc if it exists
if [[ -f "{home}/.bashrc" ]]; then
    source "{home}/.bashrc"
fi

# Run init commands
{init_script}true

# Modify prompt with green prefix
PS1="\[\033[32m\]({name})\[\033[0m\] $PS1"
"#,
            home = home,
            init_script = init_script,
            name = shell_name
        );

        let bashrc_path = temp_dir.path().join(".bashrc");
        std::fs::write(&bashrc_path, bashrc_content).map_err(|e| {
            MeriadocError::Execution(format!("failed to write temp .bashrc: {}", e))
        })?;

        // Leak the TempDir so it's not deleted before exec
        let temp_path = temp_dir.keep();
        let bashrc_path = temp_path.join(".bashrc");

        let err = Command::new(shell)
            .arg("--rcfile")
            .arg(&bashrc_path)
            .arg("-i")
            .current_dir(workdir)
            .envs(env.iter())
            .exec();

        Err(MeriadocError::Execution(format!(
            "failed to exec bash: {}",
            err
        )))
    }

    #[cfg(unix)]
    fn exec_generic(
        shell: &str,
        workdir: &Path,
        env: &ResolvedEnv,
        init_script: &str,
        _shell_name: &str,
    ) -> Result<(), MeriadocError> {
        // Fallback: just exec the shell with init commands
        let full_script = format!("{}exec {} -i", init_script, shell);
        let err = Command::new("sh")
            .arg("-c")
            .arg(&full_script)
            .current_dir(workdir)
            .envs(env.iter())
            .exec();

        Err(MeriadocError::Execution(format!(
            "failed to exec shell: {}",
            err
        )))
    }

    /// Placeholder for non-Unix platforms
    #[cfg(not(unix))]
    pub fn start(
        _workdir: &Path,
        _env: &ResolvedEnv,
        _init_cmds: &[String],
        _shell_name: &str,
        _project_name: &str,
    ) -> Result<(), MeriadocError> {
        Err(MeriadocError::Execution(
            "interactive shells are not supported on this platform".to_string(),
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellType {
    Zsh,
    Bash,
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_shell_type_zsh() {
        assert_eq!(
            InteractiveShell::detect_shell_type("/bin/zsh"),
            ShellType::Zsh
        );
        assert_eq!(
            InteractiveShell::detect_shell_type("/usr/local/bin/zsh"),
            ShellType::Zsh
        );
        assert_eq!(
            InteractiveShell::detect_shell_type("/opt/homebrew/bin/zsh-5.9"),
            ShellType::Zsh
        );
    }

    #[test]
    fn test_detect_shell_type_bash() {
        assert_eq!(
            InteractiveShell::detect_shell_type("/bin/bash"),
            ShellType::Bash
        );
        assert_eq!(
            InteractiveShell::detect_shell_type("/usr/local/bin/bash"),
            ShellType::Bash
        );
        assert_eq!(
            InteractiveShell::detect_shell_type("/opt/homebrew/bin/bash-5.2"),
            ShellType::Bash
        );
    }

    #[test]
    fn test_detect_shell_type_other() {
        assert_eq!(
            InteractiveShell::detect_shell_type("/bin/sh"),
            ShellType::Other
        );
        assert_eq!(
            InteractiveShell::detect_shell_type("/bin/fish"),
            ShellType::Other
        );
        assert_eq!(
            InteractiveShell::detect_shell_type("/usr/bin/tcsh"),
            ShellType::Other
        );
    }
}
