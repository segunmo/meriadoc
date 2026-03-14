use crate::core::spec::ConditionSpec;
use crate::core::spec::condition::FailurePolicySpec;
use crate::core::validation::{ValidationError, ValidationResult};

pub struct FailurePolicyValidator;

impl FailurePolicyValidator {
    pub fn validate(policy: &FailurePolicySpec) -> ValidationResult {
        let mut result: ValidationResult = ValidationResult::new();

        for cmd in &policy.cmds {
            if cmd.trim().is_empty() {
                result.push("", ValidationError::EmptyCommandFailurePolicy);
            }
        }

        result
    }
}

pub struct ConditionValidator;

impl ConditionValidator {
    pub fn validate(condition: &ConditionSpec) -> ValidationResult {
        let mut result: ValidationResult = ValidationResult::new();

        if condition.cmds.is_empty() {
            result.push("", ValidationError::EmptyCondition);
        }

        for cmd in &condition.cmds {
            if cmd.trim().is_empty() {
                result.push("", ValidationError::EmptyCommandCondition);
            }
        }

        if let Some(policy) = &condition.on_failure {
            let policy_result: ValidationResult = FailurePolicyValidator::validate(policy);
            for policy_error in policy_result.into_errors() {
                result.push("", policy_error.error);
            }
        }

        result
    }
}
