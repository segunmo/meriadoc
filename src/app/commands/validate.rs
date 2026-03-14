//! Validate command handler.

use crate::app::App;
use crate::cli::ValidateTarget;
use crate::core::resolver::EntityResolver;
use crate::core::validation::{MeriadocError, ProjectValidator};

pub fn handle_validate(target: Option<ValidateTarget>, app: &App) -> Result<(), MeriadocError> {
    match target {
        Some(ValidateTarget::Project { name }) => {
            // Find project by name
            let project = app
                .projects
                .iter()
                .find(|p| EntityResolver::project_name(p) == name)
                .ok_or_else(|| MeriadocError::EntityNotFound {
                    kind: "project".to_string(),
                    name: name.clone(),
                })?;

            println!("Validating project: {}", name);

            let specs: Vec<_> = project.specs.iter().map(|s| s.spec.clone()).collect();
            let result = ProjectValidator::validate(&specs);

            if result.is_ok() {
                println!("  All specs valid.");
            } else {
                println!("  Validation errors:");
                for err in result.errors() {
                    println!("    [{}] {}", err.context, err.error);
                }
            }
        }

        None => {
            // Validate all projects
            if app.projects.is_empty() {
                println!(
                    "No projects found. Use 'meriadoc config add <path>' to add a discovery root."
                );
                return Ok(());
            }

            let mut total_errors = 0;
            let mut total_specs = 0;

            for project in &app.projects {
                let project_name = EntityResolver::project_name(project);

                println!("Validating project: {}", project_name);

                let specs: Vec<_> = project.specs.iter().map(|s| s.spec.clone()).collect();
                total_specs += specs.len();

                let result = ProjectValidator::validate(&specs);

                if result.is_ok() {
                    println!("  {} spec file(s) valid.", project.specs.len());
                } else {
                    let errors = result.errors();
                    total_errors += errors.len();
                    println!("  {} error(s):", errors.len());
                    for err in errors {
                        println!("    [{}] {}", err.context, err.error);
                    }
                }
            }

            println!();
            if total_errors == 0 {
                println!(
                    "Validation complete: {} spec file(s), no errors.",
                    total_specs
                );
            } else {
                println!(
                    "Validation complete: {} spec file(s), {} error(s).",
                    total_specs, total_errors
                );
            }
        }
    }

    Ok(())
}
