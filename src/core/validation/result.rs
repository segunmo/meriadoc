use crate::core::validation::{ValidationError, error::ContextualValidationError};

#[derive(Debug, Default)]
pub struct ValidationResult {
    errors: Vec<ContextualValidationError>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn errors(&self) -> &[ContextualValidationError] {
        &self.errors
    }

    pub fn push(&mut self, context: impl Into<String>, error: ValidationError) {
        self.errors.push(ContextualValidationError {
            context: context.into(),
            error,
        });
    }

    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
    }

    pub fn into_errors(self) -> Vec<ContextualValidationError> {
        self.errors
    }
}
