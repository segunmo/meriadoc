use crate::core::spec::shell::ShellSpec;
use crate::core::validation::{EnvironmentValidator, ValidationError, ValidationResult};

pub struct ShellValidator;

impl ShellValidator {
    pub fn validate(shell_name: &str, shell: &ShellSpec) -> ValidationResult {
        let mut result: ValidationResult = ValidationResult::new();

        for cmd in &shell.init_cmds {
            if cmd.trim().is_empty() {
                result.push(
                    "",
                    ValidationError::EmptyCommandShell {
                        shell: shell_name.to_string(),
                    },
                );
            }
        }

        EnvironmentValidator::validate_map(&shell.env, format!("shell '{}'", shell_name));

        result
    }
}
