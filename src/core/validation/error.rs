use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MeriadocError {
    #[error("{0}")]
    Validation(#[from] ValidationError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("{kind} `{name}` not found")]
    EntityNotFound { kind: String, name: String },

    #[error("{kind} `{name}` is ambiguous (found in {count} projects, use project:name syntax)")]
    AmbiguousEntity {
        kind: String,
        name: String,
        count: usize,
    },

    #[error("execution failed: {0}")]
    Execution(String),

    #[error("precondition failed for {entity}: {message}")]
    PreconditionFailed { entity: String, message: String },
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("unsupported spec version `{version}` in {file}")]
    UnsupportedVersion { version: String, file: PathBuf },

    #[error("duplicate {kind} name `{name}`")]
    DuplicateName { kind: &'static str, name: String },

    #[error("task `{task}` referenced by job `{job}` does not exist")]
    UnknownTask { job: String, task: String },

    #[error("task `{task}` has no commands")]
    EmptyTask { task: String },

    #[error("condition must have at least one command")]
    EmptyCondition,

    #[error("failure policy contains empty command")]
    EmptyCommandFailurePolicy,

    #[error("condition contains empty command")]
    EmptyCommandCondition,

    #[error("task `{task}` contains empty command")]
    EmptyCommandTask { task: String },

    #[error("shell `{shell}` contains empty command")]
    EmptyCommandShell { shell: String },

    #[error("job `{job}` must reference at least one task")]
    EmptyJob { job: String },

    #[error("required env var `{var}` is missing")]
    MissingRequiredEnv { var: String },

    #[error("env var `{var}` has empty type")]
    EnvEmptyType { var: String },

    #[error("env var `{var}` has empty option value")]
    EnvEmptyOption { var: String },

    #[error("env var `{var}` default `{default}` is not in options {options:?}")]
    EnvDefaultNotInOptions {
        var: String,
        default: String,
        options: Vec<String>,
    },

    #[error("env var `{var}` has invalid value `{value}`, valid options are: {options:?}")]
    InvalidChoice {
        var: String,
        value: String,
        options: Vec<String>,
    },
}

#[derive(Debug)]
pub struct ContextualValidationError {
    pub context: String,
    pub error: ValidationError,
}
