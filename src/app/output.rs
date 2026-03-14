//! JSON output types for programmatic consumption.
//!
//! These types are designed to be consumed by AI agents and other tools
//! that integrate with Meriadoc programmatically.

use std::collections::HashMap;

use serde::Serialize;

use crate::core::spec::VarType;

/// Output wrapper for list commands
#[derive(Serialize)]
pub struct ListOutput<T> {
    pub items: Vec<T>,
    pub count: usize,
}

impl<T> ListOutput<T> {
    pub fn new(items: Vec<T>) -> Self {
        let count = items.len();
        Self { items, count }
    }
}

/// Project information for JSON output
#[derive(Serialize)]
pub struct ProjectOutput {
    pub name: String,
    pub root: String,
    pub spec_files: usize,
    pub tasks: usize,
    pub jobs: usize,
    pub shells: usize,
}

/// Task information for JSON output
#[derive(Serialize)]
pub struct TaskOutput {
    pub name: String,
    pub project: String,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workdir: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cmds: Vec<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, EnvVarOutput>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub env_files: Vec<String>,
    pub has_preconditions: bool,
    pub has_on_failure: bool,
}

/// Job information for JSON output
#[derive(Serialize)]
pub struct JobOutput {
    pub name: String,
    pub project: String,
    pub description: Option<String>,
    pub tasks: Vec<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, EnvVarOutput>,
    pub has_on_failure: bool,
}

/// Shell information for JSON output
#[derive(Serialize)]
pub struct ShellOutput {
    pub name: String,
    pub project: String,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workdir: Option<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, EnvVarOutput>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub init_cmds: Vec<String>,
}

/// Environment variable specification for JSON output
#[derive(Serialize)]
pub struct EnvVarOutput {
    #[serde(rename = "type")]
    pub var_type: VarType,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
}

/// Detailed project info for JSON output
#[derive(Serialize)]
pub struct ProjectDetailOutput {
    pub name: String,
    pub root: String,
    pub spec_files: Vec<String>,
    pub tasks: Vec<TaskSummary>,
    pub jobs: Vec<JobSummary>,
    pub shells: Vec<ShellSummary>,
}

/// Brief task summary for project detail
#[derive(Serialize)]
pub struct TaskSummary {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Brief job summary for project detail
#[derive(Serialize)]
pub struct JobSummary {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub task_count: usize,
}

/// Brief shell summary for project detail
#[derive(Serialize)]
pub struct ShellSummary {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Print JSON output to stdout
pub fn print_json<T: Serialize>(value: &T) {
    // Use pretty print for readability
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
    );
}
