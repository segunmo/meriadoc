use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported environment variable types.
///
/// These types determine how the variable is presented in UIs
/// and what validation is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum VarType {
    /// Plain text input (default)
    #[default]
    String,
    /// Numeric value (integers and decimals)
    Number,
    /// Whole numbers only
    Integer,
    /// True/false toggle
    Boolean,
    /// Selection from predefined options (requires `options` field)
    Choice,
    /// File system path (enables file browser in UI)
    Filepath,
    /// Sensitive value (masked in UI, not logged)
    Secret,
}

impl VarType {
    /// Get the string representation of the type.
    pub fn as_str(&self) -> &'static str {
        match self {
            VarType::String => "string",
            VarType::Number => "number",
            VarType::Integer => "integer",
            VarType::Boolean => "boolean",
            VarType::Choice => "choice",
            VarType::Filepath => "filepath",
            VarType::Secret => "secret",
        }
    }

    /// Check if this type should be masked in output.
    pub fn is_sensitive(&self) -> bool {
        matches!(self, VarType::Secret)
    }
}

impl fmt::Display for VarType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Environment variable specification.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnvVarSpec {
    /// The type of this variable (determines UI widget and validation)
    #[serde(rename = "type", default)]
    pub var_type: VarType,

    /// Default value if not provided
    pub default: Option<String>,

    /// Available options (required for `choice` type)
    #[serde(default)]
    pub options: Vec<String>,

    /// Whether this variable must be provided
    #[serde(default)]
    pub required: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_type_default_is_string() {
        let var_type: VarType = Default::default();
        assert_eq!(var_type, VarType::String);
    }

    #[test]
    fn test_var_type_serialization() {
        assert_eq!(
            serde_json::to_string(&VarType::String).unwrap(),
            "\"string\""
        );
        assert_eq!(
            serde_json::to_string(&VarType::Filepath).unwrap(),
            "\"filepath\""
        );
    }

    #[test]
    fn test_var_type_deserialization() {
        assert_eq!(
            serde_json::from_str::<VarType>("\"string\"").unwrap(),
            VarType::String
        );
        assert_eq!(
            serde_json::from_str::<VarType>("\"boolean\"").unwrap(),
            VarType::Boolean
        );
        assert_eq!(
            serde_json::from_str::<VarType>("\"filepath\"").unwrap(),
            VarType::Filepath
        );
    }

    #[test]
    fn test_env_var_spec_default_type() {
        let yaml = r#"
default: "hello"
"#;
        let spec: EnvVarSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(spec.var_type, VarType::String);
    }

    #[test]
    fn test_env_var_spec_with_type() {
        let yaml = r#"
type: boolean
default: "true"
"#;
        let spec: EnvVarSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(spec.var_type, VarType::Boolean);
    }

    #[test]
    fn test_is_sensitive() {
        assert!(!VarType::String.is_sensitive());
        assert!(VarType::Secret.is_sensitive());
    }
}
