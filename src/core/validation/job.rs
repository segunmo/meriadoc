use std::collections::HashSet;

use crate::core::spec::job::JobSpec;
use crate::core::validation::ValidationResult;
use crate::core::validation::env::EnvironmentValidator;
use crate::core::validation::error::ValidationError;

pub struct JobValidator;

impl JobValidator {
    pub fn validate(job_name: &str, job: &JobSpec, tasks: &HashSet<String>) -> ValidationResult {
        let mut result = ValidationResult::new();

        // A job must reference at least one task
        if job.tasks.is_empty() {
            result.push(
                "",
                ValidationError::EmptyJob {
                    job: job_name.to_string(),
                },
            );
        }

        for task in &job.tasks {
            if !tasks.contains(task) {
                result.push(
                    "",
                    ValidationError::UnknownTask {
                        job: job_name.to_string(),
                        task: task.to_string(),
                    },
                );
            }
        }

        // Validate job-level environment variables
        let env_result: ValidationResult =
            EnvironmentValidator::validate_map(&job.env, format!("job '{job_name}'"));
        result.merge(env_result);

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_job(tasks: Vec<&str>) -> JobSpec {
        JobSpec {
            description: None,
            tasks: tasks.into_iter().map(|s| s.to_string()).collect(),
            env: HashMap::new(),
            env_files: vec![],
            on_failure: None,
        }
    }

    fn make_task_set(tasks: Vec<&str>) -> HashSet<String> {
        tasks.into_iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_valid_job() {
        let job = make_job(vec!["task1", "task2"]);
        let tasks = make_task_set(vec!["task1", "task2", "task3"]);
        let result = JobValidator::validate("my_job", &job, &tasks);
        assert!(result.is_ok());
    }

    #[test]
    fn test_single_task_valid() {
        let job = make_job(vec!["task1"]);
        let tasks = make_task_set(vec!["task1"]);
        let result = JobValidator::validate("my_job", &job, &tasks);
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_job_error() {
        let job = make_job(vec![]);
        let tasks = make_task_set(vec!["task1"]);
        let result = JobValidator::validate("my_job", &job, &tasks);

        assert!(!result.is_ok());
        let errors = result.errors();
        assert!(
            errors
                .iter()
                .any(|e| matches!(&e.error, ValidationError::EmptyJob { job } if job == "my_job"))
        );
    }

    #[test]
    fn test_unknown_task_error() {
        let job = make_job(vec!["task1", "nonexistent"]);
        let tasks = make_task_set(vec!["task1", "task2"]);
        let result = JobValidator::validate("my_job", &job, &tasks);

        assert!(!result.is_ok());
        let errors = result.errors();
        assert!(errors.iter().any(|e| matches!(
            &e.error,
            ValidationError::UnknownTask { job, task }
            if job == "my_job" && task == "nonexistent"
        )));
    }

    #[test]
    fn test_multiple_unknown_tasks() {
        let job = make_job(vec!["unknown1", "unknown2"]);
        let tasks = make_task_set(vec!["task1"]);
        let result = JobValidator::validate("my_job", &job, &tasks);

        assert!(!result.is_ok());
        let errors = result.errors();
        // Should have at least 2 errors for unknown tasks
        let unknown_errors: Vec<_> = errors
            .iter()
            .filter(|e| matches!(&e.error, ValidationError::UnknownTask { .. }))
            .collect();
        assert_eq!(unknown_errors.len(), 2);
    }

    #[test]
    fn test_job_with_invalid_env() {
        use crate::core::spec::{EnvVarSpec, VarType};

        let mut job = make_job(vec!["task1"]);
        job.env.insert(
            "BAD_VAR".to_string(),
            EnvVarSpec {
                var_type: VarType::Choice, // Choice without options is invalid
                default: None,
                options: vec![],
                required: false,
            },
        );

        let tasks = make_task_set(vec!["task1"]);
        let result = JobValidator::validate("my_job", &job, &tasks);
        assert!(!result.is_ok());
    }

    #[test]
    fn test_job_with_valid_env() {
        use crate::core::spec::{EnvVarSpec, VarType};

        let mut job = make_job(vec!["task1"]);
        job.env.insert(
            "MY_VAR".to_string(),
            EnvVarSpec {
                var_type: VarType::String,
                default: Some("default".to_string()),
                options: vec![],
                required: false,
            },
        );

        let tasks = make_task_set(vec!["task1"]);
        let result = JobValidator::validate("my_job", &job, &tasks);
        assert!(result.is_ok());
    }
}
