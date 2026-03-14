pub mod commands;
pub mod dispatch;
pub mod output;

use std::path::Path;

use crate::config::MeriadocConfig;
use crate::core::resolver::EntityResolver;
use crate::core::spec::{JobSpec, ShellSpec, TaskSpec};
use crate::core::validation::MeriadocError;
use crate::repo::{Project, ProjectDiscovery, ProjectLoader, ValidationCache};

pub struct App {
    pub config: MeriadocConfig,
    pub projects: Vec<Project>,
    pub cache: ValidationCache,
}

/// Context passed to closures during task iteration.
pub struct TaskIterItem<'a> {
    pub project_name: &'a str,
    pub task_name: &'a str,
    pub task: &'a TaskSpec,
}

/// Context passed to closures during job iteration.
pub struct JobIterItem<'a> {
    pub project_name: &'a str,
    pub job_name: &'a str,
    pub job: &'a JobSpec,
}

/// Context passed to closures during shell iteration.
pub struct ShellIterItem<'a> {
    pub project_name: &'a str,
    pub shell_name: &'a str,
    pub shell: &'a ShellSpec,
}

impl App {
    pub fn new(config: MeriadocConfig) -> Result<Self, MeriadocError> {
        // Load validation cache
        let cache = ValidationCache::load(&config.cache.dir)?;

        // Discover and load projects from enabled roots
        let mut projects = Vec::new();
        for root in &config.discovery.roots {
            if !root.enabled {
                continue;
            }

            let discovered = ProjectDiscovery::discover(
                &root.path,
                &config.discovery.spec_files,
                config.discovery.max_depth,
            );

            for project in discovered {
                match ProjectLoader::load(project) {
                    Ok(loaded) => projects.push(loaded),
                    Err(e) => eprintln!("Warning: failed to load project: {}", e),
                }
            }
        }

        Ok(Self {
            config,
            projects,
            cache,
        })
    }

    /// Get the config directory parent (used for saved env storage)
    pub fn config_parent_dir(&self) -> &Path {
        self.config.cache.dir.parent().unwrap_or(Path::new("."))
    }

    /// Iterate over all tasks, calling the closure for each one.
    pub fn for_each_task<F>(&self, mut f: F)
    where
        F: FnMut(TaskIterItem<'_>),
    {
        for project in &self.projects {
            let project_name = EntityResolver::project_name(project);
            for spec in &project.specs {
                for (task_name, task) in &spec.spec.tasks {
                    f(TaskIterItem {
                        project_name,
                        task_name,
                        task,
                    });
                }
            }
        }
    }

    /// Iterate over agent-enabled tasks only (filtered by agent.enabled).
    pub fn for_each_agent_task<F>(&self, mut f: F)
    where
        F: FnMut(TaskIterItem<'_>),
    {
        self.for_each_task(|info| {
            // Skip if agent.enabled == false
            if let Some(agent) = &info.task.agent
                && !agent.enabled
            {
                return;
            }
            f(info);
        });
    }

    /// Iterate over all jobs, calling the closure for each one.
    pub fn for_each_job<F>(&self, mut f: F)
    where
        F: FnMut(JobIterItem<'_>),
    {
        for project in &self.projects {
            let project_name = EntityResolver::project_name(project);
            for spec in &project.specs {
                for (job_name, job) in &spec.spec.jobs {
                    f(JobIterItem {
                        project_name,
                        job_name,
                        job,
                    });
                }
            }
        }
    }

    /// Iterate over all shells, calling the closure for each one.
    pub fn for_each_shell<F>(&self, mut f: F)
    where
        F: FnMut(ShellIterItem<'_>),
    {
        for project in &self.projects {
            let project_name = EntityResolver::project_name(project);
            for spec in &project.specs {
                for (shell_name, shell) in &spec.spec.shells {
                    f(ShellIterItem {
                        project_name,
                        shell_name,
                        shell,
                    });
                }
            }
        }
    }
}
