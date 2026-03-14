//! Doctor command handler.

use crate::app::App;
use crate::core::validation::{MeriadocError, ProjectValidator};

pub fn handle_doctor(app: &App) -> Result<(), MeriadocError> {
    println!("Meriadoc Doctor");
    println!("===============");
    println!();

    let mut issues = 0;

    // 1. Check config
    println!("Configuration:");
    let enabled_roots: Vec<_> = app
        .config
        .discovery
        .roots
        .iter()
        .filter(|r| r.enabled)
        .collect();
    let disabled_roots: Vec<_> = app
        .config
        .discovery
        .roots
        .iter()
        .filter(|r| !r.enabled)
        .collect();

    if enabled_roots.is_empty() {
        println!("  [!] No discovery roots configured");
        println!("      Run 'meriadoc config add <path>' to add a project directory");
        issues += 1;
    } else {
        println!("  [✓] {} discovery root(s) enabled", enabled_roots.len());
    }

    if !disabled_roots.is_empty() {
        println!("  [i] {} discovery root(s) disabled", disabled_roots.len());
    }

    // 2. Check discovery roots exist
    println!();
    println!("Discovery roots:");
    for root in &enabled_roots {
        if root.path.exists() {
            println!("  [✓] {} exists", root.path.display());
        } else {
            println!("  [!] {} does not exist", root.path.display());
            issues += 1;
        }
    }

    // 3. Check projects
    println!();
    println!("Projects:");
    if app.projects.is_empty() {
        println!("  [!] No projects discovered");
        if !enabled_roots.is_empty() {
            println!(
                "      Check that spec files (meriadoc.yaml, merry.yaml) exist in your projects"
            );
        }
        issues += 1;
    } else {
        println!("  [✓] {} project(s) discovered", app.projects.len());

        let total_specs: usize = app.projects.iter().map(|p| p.specs.len()).sum();
        let total_tasks: usize = app
            .projects
            .iter()
            .flat_map(|p| &p.specs)
            .map(|s| s.spec.tasks.len())
            .sum();
        let total_jobs: usize = app
            .projects
            .iter()
            .flat_map(|p| &p.specs)
            .map(|s| s.spec.jobs.len())
            .sum();
        let total_shells: usize = app
            .projects
            .iter()
            .flat_map(|p| &p.specs)
            .map(|s| s.spec.shells.len())
            .sum();

        println!("  [i] {} spec file(s)", total_specs);
        println!(
            "  [i] {} task(s), {} job(s), {} shell(s)",
            total_tasks, total_jobs, total_shells
        );
    }

    // 4. Validate all specs
    println!();
    println!("Validation:");
    let mut validation_errors = 0;
    for project in &app.projects {
        let specs: Vec<_> = project.specs.iter().map(|s| s.spec.clone()).collect();
        let result = ProjectValidator::validate(&specs);
        if !result.is_ok() {
            validation_errors += result.errors().len();
        }
    }

    if validation_errors == 0 {
        println!("  [✓] All specs valid");
    } else {
        println!("  [!] {} validation error(s)", validation_errors);
        println!("      Run 'meriadoc validate' for details");
        issues += 1;
    }

    // 5. Check cache
    println!();
    println!("Cache:");
    if app.config.cache.enabled {
        println!("  [✓] Validation cache enabled");
        println!("  [i] {} cached entries", app.cache.len());
        println!("  [i] Cache dir: {}", app.config.cache.dir.display());
    } else {
        println!("  [i] Validation cache disabled");
    }

    // Summary
    println!();
    println!("===============");
    if issues == 0 {
        println!("No issues found. Meriadoc is ready to use.");
    } else {
        println!("{} issue(s) found.", issues);
    }

    Ok(())
}
