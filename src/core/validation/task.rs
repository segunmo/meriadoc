use crate::core::spec::task::TaskSpec;
use crate::core::validation::{
    ConditionValidator, EnvironmentValidator, ValidationError, ValidationResult,
};

pub struct TaskValidator;

impl TaskValidator {
    pub fn validate(task_name: &str, task: &TaskSpec) -> ValidationResult {
        let mut result = ValidationResult::new();
        let context = format!("task '{task_name}'");

        // Must have at least one command
        if task.cmds.is_empty() {
            result.push(
                &context,
                ValidationError::EmptyTask {
                    task: task_name.to_string(),
                },
            );
        }

        // Commands must not be empty
        for cmd in &task.cmds {
            if cmd.trim().is_empty() {
                result.push(
                    &context,
                    ValidationError::EmptyCommandTask {
                        task: task_name.to_string(),
                    },
                );
            }
        }

        // Environment validation
        let env_result = EnvironmentValidator::validate_map(&task.env, &context);
        result.merge(env_result);

        // Preconditions
        for condition in &task.preconditions {
            let condition_result = ConditionValidator::validate(condition);

            for err in condition_result.into_errors() {
                result.push(format!("{context} precondition"), err.error);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_task(cmds: Vec<&str>) -> TaskSpec {
        TaskSpec {
            description: None,
            cmds: cmds.into_iter().map(|s| s.to_string()).collect(),
            workdir: None,
            env: HashMap::new(),
            env_files: vec![],
            preconditions: vec![],
            on_failure: None,
            docs: None,
            agent: None,
        }
    }

    #[test]
    fn test_valid_task() {
        let task = make_task(vec!["echo hello", "echo world"]);
        let result = TaskValidator::validate("my_task", &task);
        assert!(result.is_ok());
    }

    #[test]
    fn test_single_command_valid() {
        let task = make_task(vec!["echo hello"]);
        let result = TaskValidator::validate("my_task", &task);
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_cmds_error() {
        let task = make_task(vec![]);
        let result = TaskValidator::validate("my_task", &task);

        assert!(!result.is_ok());
        let errors = result.errors();
        assert!(
            errors.iter().any(
                |e| matches!(&e.error, ValidationError::EmptyTask { task } if task == "my_task")
            )
        );
    }

    #[test]
    fn test_empty_command_string_error() {
        let task = make_task(vec!["echo hello", "", "echo world"]);
        let result = TaskValidator::validate("my_task", &task);

        assert!(!result.is_ok());
        let errors = result.errors();
        assert!(errors.iter().any(
            |e| matches!(&e.error, ValidationError::EmptyCommandTask { task } if task == "my_task")
        ));
    }

    #[test]
    fn test_whitespace_only_command_error() {
        let task = make_task(vec!["echo hello", "   ", "echo world"]);
        let result = TaskValidator::validate("my_task", &task);

        assert!(!result.is_ok());
    }

    #[test]
    fn test_task_with_invalid_env() {
        use crate::core::spec::{EnvVarSpec, VarType};

        let mut task = make_task(vec!["echo hello"]);
        task.env.insert(
            "BAD_VAR".to_string(),
            EnvVarSpec {
                var_type: VarType::Choice, // Choice without options is invalid
                default: None,
                options: vec![],
                required: false,
            },
        );

        let result = TaskValidator::validate("my_task", &task);
        assert!(!result.is_ok());
    }

    #[test]
    fn test_task_with_valid_env() {
        use crate::core::spec::{EnvVarSpec, VarType};

        let mut task = make_task(vec!["echo $MY_VAR"]);
        task.env.insert(
            "MY_VAR".to_string(),
            EnvVarSpec {
                var_type: VarType::String,
                default: Some("default".to_string()),
                options: vec![],
                required: false,
            },
        );

        let result = TaskValidator::validate("my_task", &task);
        assert!(result.is_ok());
    }

    #[test]
    fn test_task_with_preconditions() {
        use crate::core::spec::ConditionSpec;

        let mut task = make_task(vec!["echo main"]);
        task.preconditions.push(ConditionSpec {
            cmds: vec!["test -f file.txt".to_string()],
            on_failure: None,
        });

        let result = TaskValidator::validate("my_task", &task);
        assert!(result.is_ok());
    }

    #[test]
    fn test_task_with_empty_precondition_cmd() {
        use crate::core::spec::ConditionSpec;

        let mut task = make_task(vec!["echo main"]);
        task.preconditions.push(ConditionSpec {
            cmds: vec!["".to_string()], // Empty command in precondition
            on_failure: None,
        });

        let result = TaskValidator::validate("my_task", &task);
        assert!(!result.is_ok());
    }
}
