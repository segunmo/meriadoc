//! Environment command handler.

use std::collections::HashMap;

use serde::Serialize;

use crate::app::App;
use crate::app::output::{EnvVarOutput, ListOutput, print_json};
use crate::cli::{EnvCommand, EnvTarget};
use crate::core::resolver::EntityResolver;
use crate::core::spec::EnvVarSpec;
use crate::core::validation::MeriadocError;
use crate::repo::SavedEnvStore;

/// Environment info for JSON output
#[derive(Serialize)]
struct EnvShowOutput {
    entity_type: String,
    entity_name: String,
    project: String,
    env: HashMap<String, EnvVarOutput>,
    env_files: Vec<String>,
}

/// Saved env entry for JSON output
#[derive(Serialize)]
struct SavedEnvEntry {
    project: String,
    entity: String,
}

pub fn handle_env(command: EnvCommand, app: &App, json: bool) -> Result<(), MeriadocError> {
    match command {
        EnvCommand::Show { target, name } => handle_show(target, name, app, json),
        EnvCommand::Ls => handle_ls(app, json),
        EnvCommand::Init { target, name } => handle_init(target, name, app),
        EnvCommand::Rm { project, entity } => handle_rm(project, entity, app),
    }
}

fn handle_show(
    target: EnvTarget,
    name: String,
    app: &App,
    json: bool,
) -> Result<(), MeriadocError> {
    match target {
        EnvTarget::Task => {
            let resolved = EntityResolver::resolve_task(&name, &app.projects)?;
            if json {
                let output = EnvShowOutput {
                    entity_type: "task".to_string(),
                    entity_name: name,
                    project: EntityResolver::project_name(resolved.project).to_string(),
                    env: env_to_output(&resolved.spec.env),
                    env_files: resolved.spec.env_files.clone(),
                };
                print_json(&output);
            } else {
                println!("Environment for task: {}", name);
                println!(
                    "  Project: {}",
                    EntityResolver::project_name(resolved.project)
                );
                println!();
                print_env_table(&resolved.spec.env, &resolved.spec.env_files);
            }
        }
        EnvTarget::Job => {
            let resolved = EntityResolver::resolve_job(&name, &app.projects)?;
            if json {
                let output = EnvShowOutput {
                    entity_type: "job".to_string(),
                    entity_name: name,
                    project: EntityResolver::project_name(resolved.project).to_string(),
                    env: env_to_output(&resolved.spec.env),
                    env_files: vec![],
                };
                print_json(&output);
            } else {
                println!("Environment for job: {}", name);
                println!(
                    "  Project: {}",
                    EntityResolver::project_name(resolved.project)
                );
                println!();

                // Show job-level env
                println!("  Job-level environment:");
                if resolved.spec.env.is_empty() {
                    println!("    (none)");
                } else {
                    print_env_table(&resolved.spec.env, &[]);
                }

                // Show each task's env
                println!();
                println!("  Task environments:");
                for task_name in &resolved.spec.tasks {
                    if let Ok(task_resolved) =
                        EntityResolver::resolve_task(task_name, &app.projects)
                    {
                        println!();
                        println!("    Task: {}", task_name);
                        if task_resolved.spec.env.is_empty()
                            && task_resolved.spec.env_files.is_empty()
                        {
                            println!("      (no env vars)");
                        } else {
                            // Indent the env table for tasks
                            for (key, spec) in &task_resolved.spec.env {
                                let default_str = if spec.required && spec.default.is_none() {
                                    "(required)".to_string()
                                } else {
                                    spec.default.clone().unwrap_or_else(|| "-".to_string())
                                };
                                println!("      {} [{}]: {}", key, spec.var_type, default_str);
                            }
                            if !task_resolved.spec.env_files.is_empty() {
                                println!(
                                    "      env_files: {}",
                                    task_resolved.spec.env_files.join(", ")
                                );
                            }
                        }
                    }
                }
            }
        }
        EnvTarget::Shell => {
            let resolved = EntityResolver::resolve_shell(&name, &app.projects)?;
            if json {
                let output = EnvShowOutput {
                    entity_type: "shell".to_string(),
                    entity_name: name,
                    project: EntityResolver::project_name(resolved.project).to_string(),
                    env: env_to_output(&resolved.spec.env),
                    env_files: resolved.spec.env_files.clone(),
                };
                print_json(&output);
            } else {
                println!("Environment for shell: {}", name);
                println!(
                    "  Project: {}",
                    EntityResolver::project_name(resolved.project)
                );
                println!();
                print_env_table(&resolved.spec.env, &resolved.spec.env_files);
            }
        }
    }
    Ok(())
}

fn env_to_output(env: &HashMap<String, EnvVarSpec>) -> HashMap<String, EnvVarOutput> {
    env.iter()
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
        .collect()
}

fn handle_ls(app: &App, json: bool) -> Result<(), MeriadocError> {
    let config_dir = app
        .config
        .cache
        .dir
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let store = SavedEnvStore::new(config_dir);

    let projects = store.list_projects()?;

    if json {
        let mut items = Vec::new();
        for project in &projects {
            let entities = store.list_for_project(project)?;
            for entity in entities {
                items.push(SavedEnvEntry {
                    project: project.clone(),
                    entity,
                });
            }
        }
        print_json(&ListOutput::new(items));
    } else if projects.is_empty() {
        println!("No saved environment files.");
        println!("Run a task with missing required vars to save prompted values.");
    } else {
        println!("Saved environment files:");
        for project in &projects {
            let entities = store.list_for_project(project)?;
            for entity in &entities {
                println!("  {}:{}", project, entity);
            }
        }
    }
    Ok(())
}

fn handle_init(target: EnvTarget, name: String, app: &App) -> Result<(), MeriadocError> {
    let (project_name, env) = match target {
        EnvTarget::Task => {
            let resolved = EntityResolver::resolve_task(&name, &app.projects)?;
            (
                EntityResolver::project_name(resolved.project).to_string(),
                resolved.spec.env.clone(),
            )
        }
        EnvTarget::Job => {
            let resolved = EntityResolver::resolve_job(&name, &app.projects)?;
            (
                EntityResolver::project_name(resolved.project).to_string(),
                resolved.spec.env.clone(),
            )
        }
        EnvTarget::Shell => {
            let resolved = EntityResolver::resolve_shell(&name, &app.projects)?;
            (
                EntityResolver::project_name(resolved.project).to_string(),
                resolved.spec.env.clone(),
            )
        }
    };

    if env.is_empty() {
        println!("No environment variables defined for this entity.");
        return Ok(());
    }

    // Build template with defaults and placeholders
    let mut template = HashMap::new();
    for (key, spec) in &env {
        let value = if let Some(default) = &spec.default {
            default.clone()
        } else {
            format!("<{}>", key)
        };
        template.insert(key.clone(), value);
    }

    let config_dir = app
        .config
        .cache
        .dir
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let store = SavedEnvStore::new(config_dir);
    let path = store.save(&project_name, &name, &template)?;

    println!("Created template: {}", path.display());
    println!();
    println!("Edit the file to set your values, then run:");
    println!(
        "  meriadoc {} {}",
        match target {
            EnvTarget::Task => "task",
            EnvTarget::Job => "job",
            EnvTarget::Shell => "shell",
        },
        name
    );

    Ok(())
}

fn handle_rm(project: String, entity: String, app: &App) -> Result<(), MeriadocError> {
    let config_dir = app
        .config
        .cache
        .dir
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let store = SavedEnvStore::new(config_dir);

    if store.exists(&project, &entity) {
        store.delete(&project, &entity)?;
        println!("Deleted saved env for {}:{}", project, entity);
    } else {
        println!("No saved env found for {}:{}", project, entity);
    }
    Ok(())
}

fn print_env_table(env: &HashMap<String, EnvVarSpec>, env_files: &[String]) {
    if env.is_empty() && env_files.is_empty() {
        println!("  No environment variables defined.");
        return;
    }

    if !env.is_empty() {
        println!("  {:<20} {:<10} {:<20} OPTIONS", "NAME", "TYPE", "DEFAULT");
        println!("  {:<20} {:<10} {:<20} -------", "----", "----", "-------");

        // Sort keys for consistent output
        let mut keys: Vec<_> = env.keys().collect();
        keys.sort();

        for key in keys {
            let spec = &env[key];
            let default_str = if spec.required && spec.default.is_none() {
                "(required)".to_string()
            } else {
                spec.default.clone().unwrap_or_else(|| "-".to_string())
            };

            let options_str = if spec.options.is_empty() {
                "-".to_string()
            } else {
                spec.options.join(", ")
            };

            println!(
                "  {:<20} {:<10} {:<20} {}",
                key, spec.var_type, default_str, options_str
            );
        }
    }

    if !env_files.is_empty() {
        println!();
        println!("  Env files: {}", env_files.join(", "));
    }
}
