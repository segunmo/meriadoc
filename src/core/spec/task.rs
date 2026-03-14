use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::spec::{AgentSpec, ConditionSpec, EnvVarSpec, FailurePolicySpec};

/*
* -------------------------
* Task
* -------------------------
*/

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskSpec {
    pub description: Option<String>,

    /// Commands are executed sequentially
    pub cmds: Vec<String>,

    /// Working directory relative to project root
    pub workdir: Option<String>,

    /// Inline environment variables (highest priority for tasks)
    #[serde(default)]
    pub env: HashMap<String, EnvVarSpec>,

    /// dotenv-style env files, relative to project root
    #[serde(default, rename = "env_files")]
    pub env_files: Vec<String>,

    /// Preconditions that must succeed before execution
    #[serde(default)]
    pub preconditions: Vec<ConditionSpec>,

    /// Failure handling policy
    pub on_failure: Option<FailurePolicySpec>,

    /// Optional path to documentation
    pub docs: Option<String>,

    /// Agent configuration for AI tool exposure
    pub agent: Option<AgentSpec>,
}
