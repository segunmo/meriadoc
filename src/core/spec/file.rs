use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::spec::JobSpec;
use crate::core::spec::ShellSpec;
use crate::core::spec::TaskSpec;

/*
* -------------------------
* Top-level file model
* -------------------------
*/

/// A single Meriadoc spec file (meriadoc.yml / merry.yml)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpecFile {
    pub version: String,

    #[serde(default)]
    pub tasks: HashMap<String, TaskSpec>,

    #[serde(default)]
    pub jobs: HashMap<String, JobSpec>,

    #[serde(default)]
    pub shells: HashMap<String, ShellSpec>,
}
