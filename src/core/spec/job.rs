use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::spec::{EnvVarSpec, FailurePolicySpec};

/*
* -------------------------
* Job
* -------------------------
*/

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JobSpec {
    pub description: Option<String>,

    /// Ordered list of task names (same file, v0.1 constraint)
    pub tasks: Vec<String>,

    /// Job-level env overrides
    #[serde(default)]
    pub env: HashMap<String, EnvVarSpec>,

    #[serde(default, rename = "env_files")]
    pub env_files: Vec<String>,

    pub on_failure: Option<FailurePolicySpec>,
}
