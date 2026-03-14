//! Info command handler.

use crate::app::App;
use crate::app::output::{
    EnvVarOutput, JobOutput, JobSummary, ProjectDetailOutput, ShellOutput, ShellSummary,
    TaskOutput, TaskSummary, print_json,
};
use crate::cli::InfoTarget;
use crate::core::resolver::EntityResolver;
use crate::core::validation::MeriadocError;

pub fn handle_info(
    target: InfoTarget,
    name: String,
    app: &App,
    json: bool,
) -> Result<(), MeriadocError> {
    match target {
        InfoTarget::Task => show_task_info(&name, app, json),
        InfoTarget::Job => show_job_info(&name, app, json),
        InfoTarget::Shell => show_shell_info(&name, app, json),
        InfoTarget::Project => show_project_info(&name, app, json),
    }
}

fn show_task_info(name: &str, app: &App, json: bool) -> Result<(), MeriadocError> {
    let resolved = EntityResolver::resolve_task(name, &app.projects)?;

    if json {
        let output = TaskOutput {
            name: name.to_string(),
            project: EntityResolver::project_name(resolved.project).to_string(),
            description: resolved.spec.description.clone(),
            workdir: resolved.spec.workdir.clone(),
            cmds: resolved.spec.cmds.clone(),
            env: resolved
                .spec
                .env
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        EnvVarOutput {
                            var_type: v.var_type,
                            required: v.required,
                            default: v.default.clone(),
                            options: v.options.clone(),
                        },
                    )
                })
                .collect(),
            env_files: resolved.spec.env_files.clone(),
            has_preconditions: !resolved.spec.preconditions.is_empty(),
            has_on_failure: resolved.spec.on_failure.is_some(),
        };
        print_json(&output);
    } else {
        println!("Task: {}", name);
        if let Some(desc) = &resolved.spec.description {
            println!("  Description: {}", desc);
        }
        println!(
            "  Project: {}",
            EntityResolver::project_name(resolved.project)
        );
        println!("  Spec file: {}", resolved.spec_file.path.display());

        println!("  Commands:");
        for cmd in &resolved.spec.cmds {
            println!("    - {}", cmd);
        }

        if let Some(workdir) = &resolved.spec.workdir {
            println!("  Working directory: {}", workdir);
        }

        if !resolved.spec.env.is_empty() {
            println!("  Environment variables:");
            for (key, spec) in &resolved.spec.env {
                let default = spec.default.as_deref().unwrap_or("<none>");
                let required = if spec.required { " (required)" } else { "" };
                println!(
                    "    {} [{}]: default={}{}",
                    key, spec.var_type, default, required
                );
            }
        }

        if !resolved.spec.env_files.is_empty() {
            println!("  Env files: {}", resolved.spec.env_files.join(", "));
        }

        if !resolved.spec.preconditions.is_empty() {
            println!(
                "  Preconditions: {} condition(s)",
                resolved.spec.preconditions.len()
            );
        }

        if resolved.spec.on_failure.is_some() {
            println!("  On failure: configured");
        }
    }

    Ok(())
}

fn show_job_info(name: &str, app: &App, json: bool) -> Result<(), MeriadocError> {
    let resolved = EntityResolver::resolve_job(name, &app.projects)?;

    if json {
        let output = JobOutput {
            name: name.to_string(),
            project: EntityResolver::project_name(resolved.project).to_string(),
            description: resolved.spec.description.clone(),
            tasks: resolved.spec.tasks.clone(),
            env: resolved
                .spec
                .env
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        EnvVarOutput {
                            var_type: v.var_type,
                            required: v.required,
                            default: v.default.clone(),
                            options: v.options.clone(),
                        },
                    )
                })
                .collect(),
            has_on_failure: resolved.spec.on_failure.is_some(),
        };
        print_json(&output);
    } else {
        println!("Job: {}", name);
        if let Some(desc) = &resolved.spec.description {
            println!("  Description: {}", desc);
        }
        println!(
            "  Project: {}",
            EntityResolver::project_name(resolved.project)
        );
        println!("  Spec file: {}", resolved.spec_file.path.display());

        println!("  Tasks ({}):", resolved.spec.tasks.len());
        for task in &resolved.spec.tasks {
            println!("    - {}", task);
        }

        if !resolved.spec.env.is_empty() {
            println!("  Environment overrides:");
            for (key, spec) in &resolved.spec.env {
                let default = spec.default.as_deref().unwrap_or("<none>");
                println!("    {} = {}", key, default);
            }
        }

        if resolved.spec.on_failure.is_some() {
            println!("  On failure: configured");
        }
    }

    Ok(())
}

fn show_shell_info(name: &str, app: &App, json: bool) -> Result<(), MeriadocError> {
    let resolved = EntityResolver::resolve_shell(name, &app.projects)?;

    if json {
        let output = ShellOutput {
            name: name.to_string(),
            project: EntityResolver::project_name(resolved.project).to_string(),
            description: resolved.spec.description.clone(),
            workdir: resolved.spec.workdir.clone(),
            env: resolved
                .spec
                .env
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        EnvVarOutput {
                            var_type: v.var_type,
                            required: v.required,
                            default: v.default.clone(),
                            options: v.options.clone(),
                        },
                    )
                })
                .collect(),
            init_cmds: resolved.spec.init_cmds.clone(),
        };
        print_json(&output);
    } else {
        println!("Shell: {}", name);
        if let Some(desc) = &resolved.spec.description {
            println!("  Description: {}", desc);
        }
        println!(
            "  Project: {}",
            EntityResolver::project_name(resolved.project)
        );
        println!("  Spec file: {}", resolved.spec_file.path.display());

        if let Some(workdir) = &resolved.spec.workdir {
            println!("  Working directory: {}", workdir);
        }

        if !resolved.spec.env.is_empty() {
            println!("  Environment variables:");
            for (key, spec) in &resolved.spec.env {
                let default = spec.default.as_deref().unwrap_or("<none>");
                println!("    {} = {}", key, default);
            }
        }

        if !resolved.spec.init_cmds.is_empty() {
            println!("  Init commands:");
            for cmd in &resolved.spec.init_cmds {
                println!("    - {}", cmd);
            }
        }
    }

    Ok(())
}

fn show_project_info(name: &str, app: &App, json: bool) -> Result<(), MeriadocError> {
    let project = app
        .projects
        .iter()
        .find(|p| EntityResolver::project_name(p) == name)
        .ok_or_else(|| MeriadocError::EntityNotFound {
            kind: "project".to_string(),
            name: name.to_string(),
        })?;

    if json {
        let mut tasks = Vec::new();
        let mut jobs = Vec::new();
        let mut shells = Vec::new();

        for spec in &project.specs {
            for (task_name, task) in &spec.spec.tasks {
                tasks.push(TaskSummary {
                    name: task_name.clone(),
                    description: task.description.clone(),
                });
            }
            for (job_name, job) in &spec.spec.jobs {
                jobs.push(JobSummary {
                    name: job_name.clone(),
                    description: job.description.clone(),
                    task_count: job.tasks.len(),
                });
            }
            for (shell_name, shell) in &spec.spec.shells {
                shells.push(ShellSummary {
                    name: shell_name.clone(),
                    description: shell.description.clone(),
                });
            }
        }

        let output = ProjectDetailOutput {
            name: EntityResolver::project_name(project).to_string(),
            root: project.root.display().to_string(),
            spec_files: project
                .specs
                .iter()
                .map(|s| s.path.display().to_string())
                .collect(),
            tasks,
            jobs,
            shells,
        };
        print_json(&output);
    } else {
        let project_name = EntityResolver::project_name(project);

        println!("Project: {}", project_name);
        println!("  Root: {}", project.root.display());
        println!("  Spec files: {}", project.specs.len());

        let mut task_count = 0;
        let mut job_count = 0;
        let mut shell_count = 0;

        for spec in &project.specs {
            task_count += spec.spec.tasks.len();
            job_count += spec.spec.jobs.len();
            shell_count += spec.spec.shells.len();
        }

        println!("  Tasks: {}", task_count);
        println!("  Jobs: {}", job_count);
        println!("  Shells: {}", shell_count);

        if task_count > 0 {
            println!("\n  Available tasks:");
            for spec in &project.specs {
                for (task_name, task) in &spec.spec.tasks {
                    let desc = task.description.as_deref().unwrap_or("");
                    if desc.is_empty() {
                        println!("    - {}", task_name);
                    } else {
                        println!("    - {} - {}", task_name, desc);
                    }
                }
            }
        }

        if job_count > 0 {
            println!("\n  Available jobs:");
            for spec in &project.specs {
                for (job_name, job) in &spec.spec.jobs {
                    let desc = job.description.as_deref().unwrap_or("");
                    if desc.is_empty() {
                        println!("    - {} ({} tasks)", job_name, job.tasks.len());
                    } else {
                        println!("    - {} ({} tasks) - {}", job_name, job.tasks.len(), desc);
                    }
                }
            }
        }

        if shell_count > 0 {
            println!("\n  Available shells:");
            for spec in &project.specs {
                for (shell_name, shell) in &spec.spec.shells {
                    let desc = shell.description.as_deref().unwrap_or("");
                    if desc.is_empty() {
                        println!("    - {}", shell_name);
                    } else {
                        println!("    - {} - {}", shell_name, desc);
                    }
                }
            }
        }
    }

    Ok(())
}
