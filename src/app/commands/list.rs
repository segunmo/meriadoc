//! List command handler.

use crate::app::App;
use crate::app::output::{
    EnvVarOutput, JobOutput, ListOutput, ProjectOutput, ShellOutput, TaskOutput, print_json,
};
use crate::cli::ListTarget;
use crate::core::resolver::EntityResolver;
use crate::core::validation::MeriadocError;

pub fn handle_ls(target: Option<ListTarget>, app: &App, json: bool) -> Result<(), MeriadocError> {
    match target {
        None | Some(ListTarget::Projects) => list_projects(app, json),
        Some(ListTarget::Tasks) => list_tasks(app, json),
        Some(ListTarget::Jobs) => list_jobs(app, json),
        Some(ListTarget::Shells) => list_shells(app, json),
    }
    Ok(())
}

fn list_projects(app: &App, json: bool) {
    if json {
        let items: Vec<ProjectOutput> = app
            .projects
            .iter()
            .map(|project| {
                let name = EntityResolver::project_name(project);
                let mut tasks = 0;
                let mut jobs = 0;
                let mut shells = 0;
                for spec in &project.specs {
                    tasks += spec.spec.tasks.len();
                    jobs += spec.spec.jobs.len();
                    shells += spec.spec.shells.len();
                }
                ProjectOutput {
                    name: name.to_string(),
                    root: project.root.display().to_string(),
                    spec_files: project.specs.len(),
                    tasks,
                    jobs,
                    shells,
                }
            })
            .collect();
        print_json(&ListOutput::new(items));
    } else {
        if app.projects.is_empty() {
            println!(
                "No projects found. Use 'meriadoc config add <path>' to add a discovery root."
            );
            return;
        }
        println!("Projects:");
        for project in &app.projects {
            let name = EntityResolver::project_name(project);
            println!("  {} ({})", name, project.root.display());
            println!("    Spec files: {}", project.specs.len());
        }
    }
}

fn list_tasks(app: &App, json: bool) {
    if json {
        let mut items: Vec<TaskOutput> = Vec::new();
        app.for_each_task(|info| {
            items.push(TaskOutput {
                name: info.task_name.to_string(),
                project: info.project_name.to_string(),
                description: info.task.description.clone(),
                workdir: info.task.workdir.clone(),
                cmds: info.task.cmds.clone(),
                env: info
                    .task
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
                env_files: info.task.env_files.clone(),
                has_preconditions: !info.task.preconditions.is_empty(),
                has_on_failure: info.task.on_failure.is_some(),
            });
        });
        print_json(&ListOutput::new(items));
    } else {
        println!("Tasks:");
        app.for_each_task(|info| {
            let desc = info.task.description.as_deref().unwrap_or("");
            if desc.is_empty() {
                println!("  {}:{}", info.project_name, info.task_name);
            } else {
                println!("  {}:{} - {}", info.project_name, info.task_name, desc);
            }
        });
    }
}

fn list_jobs(app: &App, json: bool) {
    if json {
        let mut items: Vec<JobOutput> = Vec::new();
        app.for_each_job(|info| {
            items.push(JobOutput {
                name: info.job_name.to_string(),
                project: info.project_name.to_string(),
                description: info.job.description.clone(),
                tasks: info.job.tasks.clone(),
                env: info
                    .job
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
                has_on_failure: info.job.on_failure.is_some(),
            });
        });
        print_json(&ListOutput::new(items));
    } else {
        println!("Jobs:");
        app.for_each_job(|info| {
            let desc = info.job.description.as_deref().unwrap_or("");
            let task_count = info.job.tasks.len();
            if desc.is_empty() {
                println!(
                    "  {}:{} ({} tasks)",
                    info.project_name, info.job_name, task_count
                );
            } else {
                println!(
                    "  {}:{} ({} tasks) - {}",
                    info.project_name, info.job_name, task_count, desc
                );
            }
        });
    }
}

fn list_shells(app: &App, json: bool) {
    if json {
        let mut items: Vec<ShellOutput> = Vec::new();
        app.for_each_shell(|info| {
            items.push(ShellOutput {
                name: info.shell_name.to_string(),
                project: info.project_name.to_string(),
                description: info.shell.description.clone(),
                workdir: info.shell.workdir.clone(),
                env: info
                    .shell
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
                init_cmds: info.shell.init_cmds.clone(),
            });
        });
        print_json(&ListOutput::new(items));
    } else {
        println!("Shells:");
        app.for_each_shell(|info| {
            let desc = info.shell.description.as_deref().unwrap_or("");
            if desc.is_empty() {
                println!("  {}:{}", info.project_name, info.shell_name);
            } else {
                println!("  {}:{} - {}", info.project_name, info.shell_name, desc);
            }
        });
    }
}
