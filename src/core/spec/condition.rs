use serde::{Deserialize, Serialize};

/*
* -------------------------
* Conditions & failure
* -------------------------
*/

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConditionSpec {
    pub cmds: Vec<String>,

    pub on_failure: Option<FailurePolicySpec>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FailurePolicySpec {
    pub r#continue: bool,

    #[serde(default)]
    pub cmds: Vec<String>,
}
