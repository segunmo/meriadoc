//! Agent configuration for AI tool exposure.

use serde::{Deserialize, Serialize};

/// Agent-specific configuration for a task.
///
/// Controls how tasks are exposed to AI agents via MCP.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AgentSpec {
    /// Whether this task is exposed to AI agents (default: true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Risk level for agent decision-making
    #[serde(default)]
    pub risk_level: RiskLevel,

    /// Confirmation message shown before execution
    pub confirmation: Option<String>,

    /// Explicitly require approval before execution (default: false)
    /// When true, task requires human confirmation regardless of risk level.
    /// When false, approval is required only for high/critical risk tasks.
    #[serde(default)]
    pub requires_approval: bool,
}

fn default_enabled() -> bool {
    true
}

/// Risk level classification for agent tasks.
///
/// Helps AI agents make informed decisions about task execution:
/// - `Low`: Safe operations (read-only, local changes)
/// - `Medium`: Reversible operations (create files, start services)
/// - `High`: Hard to reverse (deploy, database migrations)
/// - `Critical`: Destructive (delete data, production changes)
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    #[default]
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    /// Get the string representation of the risk level.
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Low => "low",
            RiskLevel::Medium => "medium",
            RiskLevel::High => "high",
            RiskLevel::Critical => "critical",
        }
    }
}
