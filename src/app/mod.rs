pub mod commands;
pub mod dispatch;
pub mod output;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::MeriadocConfig;
use crate::core::resolver::EntityResolver;
use crate::core::spec::{JobSpec, ShellSpec, TaskSpec};
use crate::core::validation::MeriadocError;
use crate::repo::{project_cache_dir, Project, ProjectDiscovery, ProjectLoader, ValidationCache};

pub struct App {
    pub config: MeriadocConfig,
    pub projects: Vec<Project>,
    /// Per-project validation caches, keyed by project root path.
    pub caches: HashMap<PathBuf, ValidationCache>,
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

        // Load per-project validation caches
        let mut caches = HashMap::new();
        if config.cache.enabled {
            for project in &projects {
                let cache_dir = project_cache_dir(&config.cache.dir, &project.root);
                let cache = ValidationCache::load(&cache_dir).unwrap_or_default();
                caches.insert(project.root.clone(), cache);
            }
        }

        Ok(Self {
            config,
            projects,
            caches,
        })
    }

    /// Get the config directory (used for saved env storage and other user data).
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
