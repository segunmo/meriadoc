use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::spec::EnvVarSpec;

/*
* -------------------------
* Shell
* -------------------------
*/

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShellSpec {
    pub description: Option<String>,

    /// Working directory for shell startup
    pub workdir: Option<String>,

    #[serde(default)]
    pub env: HashMap<String, EnvVarSpec>,

    #[serde(default, rename = "env_files")]
    pub env_files: Vec<String>,

    /// Commands executed before handing control to the user
    #[serde(default)]
    pub init_cmds: Vec<String>,
}
