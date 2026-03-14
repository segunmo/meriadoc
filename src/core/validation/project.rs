use std::collections::HashSet;

use crate::core::spec::file::SpecFile;
use crate::core::validation::error::ValidationError;
use crate::core::validation::{JobValidator, ShellValidator, TaskValidator, ValidationResult};

pub struct ProjectValidator;

impl ProjectValidator {
    pub fn validate(specs: &[SpecFile]) -> ValidationResult {
        let mut task_names: HashSet<String> = HashSet::new();
        let mut job_names: HashSet<String> = HashSet::new();
        let mut shell_names: HashSet<String> = HashSet::new();
        let mut result: ValidationResult = ValidationResult::new();

        // 1. Global name uniqueness
        for spec in specs {
            for name in spec.tasks.keys() {
                if !task_names.insert(name.clone()) {
                    result.push(
                        "",
                        ValidationError::DuplicateName {
                            kind: "task",
                            name: name.clone(),
                        },
                    );
                }
            }

            for name in spec.jobs.keys() {
                if !job_names.insert(name.clone()) {
                    result.push(
                        "",
                        ValidationError::DuplicateName {
                            kind: "job",
                            name: name.clone(),
                        },
                    );
                }
            }

            for name in spec.shells.keys() {
                if !shell_names.insert(name.clone()) {
                    result.push(
                        "",
                        ValidationError::DuplicateName {
                            kind: "shell",
                            name: name.clone(),
                        },
                    );
                }
            }
        }

        // 2. Task validation
        for spec in specs {
            for (task_name, task) in &spec.tasks {
                let task_result = TaskValidator::validate(task_name, task);
                result.merge(task_result);
            }
        }

        // 3. Job validation
        for spec in specs {
            let task_names: HashSet<String> = spec.tasks.keys().cloned().collect();

            for (job_name, job) in &spec.jobs {
                let job_result = JobValidator::validate(job_name, job, &task_names);
                result.merge(job_result);
            }
        }
        // 4. Shell validation
        for spec in specs {
            for (shell_name, shell) in &spec.shells {
                let shell_result = ShellValidator::validate(shell_name, shell);
                result.merge(shell_result);
            }
        }

        result
    }
}
