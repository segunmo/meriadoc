use crate::core::spec::env::{EnvVarSpec, VarType};
use crate::core::validation::{ValidationError, ValidationResult};
use std::collections::HashMap;

pub struct EnvironmentValidator;

impl EnvironmentValidator {
    pub fn validate(name: &str, spec: &EnvVarSpec) -> ValidationResult {
        let mut result = ValidationResult::new();

        // VarType is now an enum, so invalid types fail at deserialization.
        // We only validate semantic constraints here.

        // Choice type should have options
        if spec.var_type == VarType::Choice && spec.options.is_empty() {
            result.push(
                "",
                ValidationError::EnvEmptyType {
                    var: name.to_string(),
                },
            );
        }

        for opt in &spec.options {
            if opt.trim().is_empty() {
                result.push(
                    "",
                    ValidationError::EnvEmptyOption {
                        var: name.to_string(),
                    },
                );
            }
        }

        if !spec.options.is_empty()
            && let Some(default) = &spec.default
            && !spec.options.contains(default)
        {
            result.push(
                "",
                ValidationError::EnvDefaultNotInOptions {
                    var: name.to_string(),
                    default: default.clone(),
                    options: spec.options.clone(),
                },
            );
        }

        // Note: We don't validate that required vars have defaults here.
        // Required vars without defaults will be prompted for at runtime
        // (in interactive mode) or fail with a clear error (in non-interactive mode).

        result
    }

    pub fn validate_map(
        env: &HashMap<String, EnvVarSpec>,
        context: impl Into<String>,
    ) -> ValidationResult {
        let context: String = context.into();
        let mut result: ValidationResult = ValidationResult::new();

        for (name, spec) in env {
            let validation_result = Self::validate(name, spec);

            for contextual_error in validation_result.into_errors() {
                result.push(format!("{context} env `{name}`"), contextual_error.error);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_spec(
        var_type: VarType,
        default: Option<&str>,
        options: Vec<&str>,
        required: bool,
    ) -> EnvVarSpec {
        EnvVarSpec {
            var_type,
            default: default.map(|s| s.to_string()),
            options: options.into_iter().map(|s| s.to_string()).collect(),
            required,
        }
    }

    #[test]
    fn test_valid_simple_env() {
        let spec = make_spec(VarType::String, Some("default"), vec![], false);
        let result = EnvironmentValidator::validate("MY_VAR", &spec);
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_with_options() {
        let spec = make_spec(
            VarType::Choice,
            Some("opt1"),
            vec!["opt1", "opt2", "opt3"],
            false,
        );
        let result = EnvironmentValidator::validate("MY_VAR", &spec);
        assert!(result.is_ok());
    }

    #[test]
    fn test_choice_without_options_error() {
        // Choice type without options is an error
        let spec = make_spec(VarType::Choice, Some("default"), vec![], false);
        let result = EnvironmentValidator::validate("MY_VAR", &spec);

        assert!(!result.is_ok());
        let errors = result.errors();
        assert!(
            errors.iter().any(
                |e| matches!(&e.error, ValidationError::EnvEmptyType { var } if var == "MY_VAR")
            )
        );
    }

    #[test]
    fn test_empty_option_error() {
        let spec = make_spec(VarType::Choice, Some("opt1"), vec!["opt1", ""], false);
        let result = EnvironmentValidator::validate("MY_VAR", &spec);

        assert!(!result.is_ok());
        let errors = result.errors();
        assert!(errors.iter().any(
            |e| matches!(&e.error, ValidationError::EnvEmptyOption { var } if var == "MY_VAR")
        ));
    }

    #[test]
    fn test_default_not_in_options_error() {
        let spec = make_spec(
            VarType::Choice,
            Some("invalid"),
            vec!["opt1", "opt2"],
            false,
        );
        let result = EnvironmentValidator::validate("MY_VAR", &spec);

        assert!(!result.is_ok());
        let errors = result.errors();
        assert!(errors.iter().any(|e| matches!(
            &e.error,
            ValidationError::EnvDefaultNotInOptions { var, default, .. }
            if var == "MY_VAR" && default == "invalid"
        )));
    }

    #[test]
    fn test_required_without_default_ok() {
        // Required without default is now valid at spec level.
        // Runtime will prompt for the value or fail with clear error.
        let spec = make_spec(VarType::String, None, vec![], true);
        let result = EnvironmentValidator::validate("MY_VAR", &spec);
        assert!(result.is_ok());
    }

    #[test]
    fn test_required_with_default_ok() {
        let spec = make_spec(VarType::String, Some("default"), vec![], true);
        let result = EnvironmentValidator::validate("MY_VAR", &spec);
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_options_no_default_ok() {
        // When there are no options, default doesn't need to be in options
        let spec = make_spec(VarType::String, Some("any_value"), vec![], false);
        let result = EnvironmentValidator::validate("MY_VAR", &spec);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_map_empty() {
        let env: HashMap<String, EnvVarSpec> = HashMap::new();
        let result = EnvironmentValidator::validate_map(&env, "test context");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_map_with_errors() {
        let mut env = HashMap::new();
        // Choice without options is an error
        env.insert(
            "VAR1".to_string(),
            make_spec(VarType::Choice, None, vec![], false),
        );
        // Default not in options
        env.insert(
            "VAR2".to_string(),
            make_spec(
                VarType::Choice,
                Some("invalid"),
                vec!["opt1", "opt2"],
                false,
            ),
        );

        let result = EnvironmentValidator::validate_map(&env, "task");
        assert!(!result.is_ok());
        assert!(result.errors().len() >= 2);
    }
}
