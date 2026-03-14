//! CLI prompting for user input.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};

use crate::core::execution::MissingVar;
use crate::core::spec::EnvVarSpec;
use crate::core::validation::MeriadocError;

/// Result of prompting for environment variables
pub struct PromptResult {
    /// The collected values
    pub values: HashMap<String, String>,
    /// Whether the user wants to save these values
    pub save: bool,
}

/// A variable with its current value for review/editing
pub struct EditableVar {
    pub name: String,
    pub spec: EnvVarSpec,
    pub current_value: Option<String>,
}

/// Trait for prompting users for environment variable values.
/// Different UI layers (CLI, TUI, GUI) can implement this differently.
pub trait EnvPrompter {
    /// Prompt for all missing variables
    fn prompt_missing(&mut self, missing: &[MissingVar]) -> Result<PromptResult, MeriadocError>;

    /// Prompt for all variables, showing current values for review/edit
    fn prompt_all(&mut self, vars: &[EditableVar]) -> Result<PromptResult, MeriadocError>;
}

/// CLI-based prompter using stdin/stdout
pub struct CliPrompter {
    /// Input stream
    input: Box<dyn BufRead>,
    /// Output stream
    output: Box<dyn Write>,
}

impl Default for CliPrompter {
    fn default() -> Self {
        Self::new()
    }
}

impl CliPrompter {
    /// Create a new CLI prompter using stdin/stdout
    pub fn new() -> Self {
        Self {
            input: Box::new(io::stdin().lock()),
            output: Box::new(io::stdout()),
        }
    }

    /// Create a CLI prompter with custom streams (for testing)
    #[cfg(test)]
    pub fn with_streams(input: Box<dyn BufRead>, output: Box<dyn Write>) -> Self {
        Self { input, output }
    }

    fn read_line(&mut self) -> Result<String, MeriadocError> {
        let mut line = String::new();
        self.input.read_line(&mut line)?;
        Ok(line.trim().to_string())
    }

    fn prompt_single(&mut self, var: &MissingVar) -> Result<String, MeriadocError> {
        self.prompt_var(&var.name, &var.spec, None)
    }

    fn prompt_editable(&mut self, var: &EditableVar) -> Result<String, MeriadocError> {
        self.prompt_var(&var.name, &var.spec, var.current_value.as_deref())
    }

    fn prompt_var(
        &mut self,
        name: &str,
        spec: &EnvVarSpec,
        current_value: Option<&str>,
    ) -> Result<String, MeriadocError> {
        if !spec.options.is_empty() {
            self.prompt_choice(name, spec, current_value)
        } else {
            self.prompt_freeform(name, spec, current_value)
        }
    }

    fn prompt_choice(
        &mut self,
        name: &str,
        spec: &EnvVarSpec,
        current_value: Option<&str>,
    ) -> Result<String, MeriadocError> {
        writeln!(self.output)?;
        writeln!(self.output, "  {} ({}) - choose from:", name, spec.var_type)?;

        // Find the current/default index for display
        let effective_current = current_value.or(spec.default.as_deref());

        for (i, option) in spec.options.iter().enumerate() {
            let marker = if Some(option.as_str()) == effective_current {
                if current_value.is_some() {
                    " (current)"
                } else {
                    " (default)"
                }
            } else {
                ""
            };
            writeln!(self.output, "    {}. {}{}", i + 1, option, marker)?;
        }

        write!(self.output, "  Select [1-{}]: ", spec.options.len())?;
        self.output.flush()?;

        let input = self.read_line()?;

        // If empty, use current value or default
        if input.is_empty() {
            if let Some(current) = current_value {
                return Ok(current.to_string());
            }
            if let Some(default) = &spec.default {
                return Ok(default.clone());
            }
            // No current or default, ask again
            return self.prompt_choice(name, spec, current_value);
        }

        // Parse as number
        if let Ok(n) = input.parse::<usize>()
            && n >= 1
            && n <= spec.options.len()
        {
            return Ok(spec.options[n - 1].clone());
        }

        // Or accept the value directly if it's in options
        if spec.options.contains(&input) {
            return Ok(input);
        }

        // Invalid input, show error and retry
        writeln!(
            self.output,
            "  Invalid selection. Please enter 1-{} or one of: {}",
            spec.options.len(),
            spec.options.join(", ")
        )?;
        self.prompt_choice(name, spec, current_value)
    }

    fn prompt_freeform(
        &mut self,
        name: &str,
        spec: &EnvVarSpec,
        current_value: Option<&str>,
    ) -> Result<String, MeriadocError> {
        let type_hint = spec.var_type.as_str();

        // Show current value hint if available
        if let Some(current) = current_value {
            // Mask sensitive values
            let display_value = if Self::is_sensitive(name) {
                Self::mask_value(current)
            } else {
                current.to_string()
            };
            write!(
                self.output,
                "  {} ({}) [current: {}]: ",
                name, type_hint, display_value
            )?;
        } else if let Some(default) = &spec.default {
            write!(
                self.output,
                "  {} ({}) [default: {}]: ",
                name, type_hint, default
            )?;
        } else {
            write!(self.output, "  {} ({}): ", name, type_hint)?;
        }
        self.output.flush()?;

        let input = self.read_line()?;

        // If empty, use current value or default
        if input.is_empty() {
            if let Some(current) = current_value {
                return Ok(current.to_string());
            }
            if let Some(default) = &spec.default {
                return Ok(default.clone());
            }
            // Required with no current or default - must provide value
            if spec.required {
                writeln!(
                    self.output,
                    "  This variable is required. Please enter a value."
                )?;
                return self.prompt_freeform(name, spec, current_value);
            }
        }

        Ok(input)
    }

    /// Check if a variable name suggests it contains sensitive data
    fn is_sensitive(name: &str) -> bool {
        let name_lower = name.to_lowercase();
        name_lower.contains("key")
            || name_lower.contains("secret")
            || name_lower.contains("password")
            || name_lower.contains("token")
            || name_lower.contains("credential")
    }

    /// Mask a sensitive value for display
    fn mask_value(value: &str) -> String {
        if value.len() <= 4 {
            "*".repeat(value.len())
        } else {
            format!("{}***", &value[..4])
        }
    }

    fn prompt_save(&mut self) -> Result<bool, MeriadocError> {
        writeln!(self.output)?;
        write!(self.output, "Save these values for future runs? [Y/n]: ")?;
        self.output.flush()?;

        let save_input = self.read_line()?;
        Ok(save_input.is_empty() || save_input.to_lowercase().starts_with('y'))
    }
}

impl EnvPrompter for CliPrompter {
    fn prompt_missing(&mut self, missing: &[MissingVar]) -> Result<PromptResult, MeriadocError> {
        writeln!(self.output)?;
        writeln!(self.output, "Missing required environment variables:")?;

        let mut values = HashMap::new();
        for var in missing {
            let value = self.prompt_single(var)?;
            writeln!(self.output, "  -> {}", value)?;
            values.insert(var.name.clone(), value);
        }

        let save = self.prompt_save()?;
        Ok(PromptResult { values, save })
    }

    fn prompt_all(&mut self, vars: &[EditableVar]) -> Result<PromptResult, MeriadocError> {
        writeln!(self.output)?;
        writeln!(
            self.output,
            "Review/edit environment variables (press Enter to keep current value):"
        )?;

        let mut values = HashMap::new();
        for var in vars {
            let value = self.prompt_editable(var)?;

            // Show confirmation
            let display = if Self::is_sensitive(&var.name) {
                Self::mask_value(&value)
            } else {
                value.clone()
            };

            // Only show "-> value" if it changed or was newly set
            let changed = var.current_value.as_deref() != Some(&value);
            if changed {
                writeln!(self.output, "  -> {}", display)?;
            }

            values.insert(var.name.clone(), value);
        }

        let save = self.prompt_save()?;
        Ok(PromptResult { values, save })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::spec::VarType;
    use std::io::Cursor;

    fn make_var(
        name: &str,
        var_type: VarType,
        default: Option<&str>,
        options: &[&str],
    ) -> MissingVar {
        MissingVar {
            name: name.to_string(),
            spec: EnvVarSpec {
                var_type,
                default: default.map(|s| s.to_string()),
                options: options.iter().map(|s| s.to_string()).collect(),
                required: true,
            },
        }
    }

    fn make_editable(
        name: &str,
        var_type: VarType,
        default: Option<&str>,
        options: &[&str],
        current: Option<&str>,
    ) -> EditableVar {
        EditableVar {
            name: name.to_string(),
            spec: EnvVarSpec {
                var_type,
                default: default.map(|s| s.to_string()),
                options: options.iter().map(|s| s.to_string()).collect(),
                required: true,
            },
            current_value: current.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_prompt_single_string() {
        let input = Cursor::new("my-value\n");
        let output = Vec::new();
        let mut prompter = CliPrompter::with_streams(Box::new(input), Box::new(output));

        let var = make_var("API_KEY", VarType::String, None, &[]);
        let result = prompter.prompt_single(&var).unwrap();
        assert_eq!(result, "my-value");
    }

    #[test]
    fn test_prompt_single_choice_by_number() {
        let input = Cursor::new("2\n");
        let output = Vec::new();
        let mut prompter = CliPrompter::with_streams(Box::new(input), Box::new(output));

        let var = make_var(
            "ENV",
            VarType::Choice,
            Some("dev"),
            &["dev", "staging", "prod"],
        );
        let result = prompter.prompt_single(&var).unwrap();
        assert_eq!(result, "staging");
    }

    #[test]
    fn test_prompt_single_choice_by_name() {
        let input = Cursor::new("prod\n");
        let output = Vec::new();
        let mut prompter = CliPrompter::with_streams(Box::new(input), Box::new(output));

        let var = make_var(
            "ENV",
            VarType::Choice,
            Some("dev"),
            &["dev", "staging", "prod"],
        );
        let result = prompter.prompt_single(&var).unwrap();
        assert_eq!(result, "prod");
    }

    #[test]
    fn test_prompt_single_choice_default() {
        let input = Cursor::new("\n");
        let output = Vec::new();
        let mut prompter = CliPrompter::with_streams(Box::new(input), Box::new(output));

        let var = make_var(
            "ENV",
            VarType::Choice,
            Some("staging"),
            &["dev", "staging", "prod"],
        );
        let result = prompter.prompt_single(&var).unwrap();
        assert_eq!(result, "staging");
    }

    #[test]
    fn test_prompt_single_string_default() {
        let input = Cursor::new("\n");
        let output = Vec::new();
        let mut prompter = CliPrompter::with_streams(Box::new(input), Box::new(output));

        let var = make_var("API_KEY", VarType::String, Some("default-key"), &[]);
        let result = prompter.prompt_single(&var).unwrap();
        assert_eq!(result, "default-key");
    }

    #[test]
    fn test_prompt_editable_keeps_current() {
        let input = Cursor::new("\n");
        let output = Vec::new();
        let mut prompter = CliPrompter::with_streams(Box::new(input), Box::new(output));

        let var = make_editable(
            "ENV",
            VarType::Choice,
            Some("dev"),
            &["dev", "staging", "prod"],
            Some("staging"),
        );
        let result = prompter.prompt_editable(&var).unwrap();
        assert_eq!(result, "staging"); // Keeps current, not default
    }

    #[test]
    fn test_prompt_editable_can_change() {
        let input = Cursor::new("3\n");
        let output = Vec::new();
        let mut prompter = CliPrompter::with_streams(Box::new(input), Box::new(output));

        let var = make_editable(
            "ENV",
            VarType::Choice,
            Some("dev"),
            &["dev", "staging", "prod"],
            Some("staging"),
        );
        let result = prompter.prompt_editable(&var).unwrap();
        assert_eq!(result, "prod");
    }

    #[test]
    fn test_is_sensitive() {
        assert!(CliPrompter::is_sensitive("API_KEY"));
        assert!(CliPrompter::is_sensitive("secret_token"));
        assert!(CliPrompter::is_sensitive("PASSWORD"));
        assert!(!CliPrompter::is_sensitive("ENV"));
        assert!(!CliPrompter::is_sensitive("DEBUG"));
    }

    #[test]
    fn test_mask_value() {
        assert_eq!(CliPrompter::mask_value("abc"), "***");
        assert_eq!(CliPrompter::mask_value("abcdef"), "abcd***");
        assert_eq!(CliPrompter::mask_value("sk-1234567890"), "sk-1***");
    }
}
