use crate::core::spec::{JobSpec, ShellSpec, TaskSpec};
use crate::core::validation::MeriadocError;
use crate::repo::project::{LoadedSpec, Project};

/// Result of resolving a task by name
#[derive(Debug, Clone)]
pub struct ResolvedTask<'a> {
    #[allow(dead_code)] // Used in tests and for future logging
    pub name: String,
    pub spec: &'a TaskSpec,
    pub spec_file: &'a LoadedSpec,
    pub project: &'a Project,
}

/// Result of resolving a job by name
#[derive(Debug, Clone)]
pub struct ResolvedJob<'a> {
    #[allow(dead_code)] // Used in tests and for future logging
    pub name: String,
    pub spec: &'a JobSpec,
    pub spec_file: &'a LoadedSpec,
    pub project: &'a Project,
}

/// Result of resolving a shell by name
#[derive(Debug, Clone)]
pub struct ResolvedShell<'a> {
    #[allow(dead_code)] // Used in tests and for future logging
    pub name: String,
    pub spec: &'a ShellSpec,
    pub spec_file: &'a LoadedSpec,
    pub project: &'a Project,
}

pub struct EntityResolver;

impl EntityResolver {
    /// Parse "project:entity" or just "entity"
    fn parse_qualified_name(name: &str) -> (Option<&str>, &str) {
        match name.split_once(':') {
            Some((project, entity)) => (Some(project), entity),
            None => (None, name),
        }
    }

    /// Get project name from path
    pub fn project_name(project: &Project) -> &str {
        project
            .root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
    }

    /// Resolve a task by name, searching all projects
    pub fn resolve_task<'a>(
        name: &str,
        projects: &'a [Project],
    ) -> Result<ResolvedTask<'a>, MeriadocError> {
        let (project_filter, task_name) = Self::parse_qualified_name(name);
        let mut matches: Vec<ResolvedTask<'a>> = Vec::new();

        for project in projects {
            // Filter by project name if qualified
            if let Some(filter) = project_filter
                && Self::project_name(project) != filter
            {
                continue;
            }

            for spec_file in &project.specs {
                if let Some(task_spec) = spec_file.spec.tasks.get(task_name) {
                    matches.push(ResolvedTask {
                        name: task_name.to_string(),
                        spec: task_spec,
                        spec_file,
                        project,
                    });
                }
            }
        }

        Self::resolve_single(matches, "task", name)
    }

    /// Resolve a job by name, searching all projects
    pub fn resolve_job<'a>(
        name: &str,
        projects: &'a [Project],
    ) -> Result<ResolvedJob<'a>, MeriadocError> {
        let (project_filter, job_name) = Self::parse_qualified_name(name);
        let mut matches: Vec<ResolvedJob<'a>> = Vec::new();

        for project in projects {
            if let Some(filter) = project_filter
                && Self::project_name(project) != filter
            {
                continue;
            }

            for spec_file in &project.specs {
                if let Some(job_spec) = spec_file.spec.jobs.get(job_name) {
                    matches.push(ResolvedJob {
                        name: job_name.to_string(),
                        spec: job_spec,
                        spec_file,
                        project,
                    });
                }
            }
        }

        Self::resolve_single(matches, "job", name)
    }

    /// Resolve a shell by name, searching all projects
    pub fn resolve_shell<'a>(
        name: &str,
        projects: &'a [Project],
    ) -> Result<ResolvedShell<'a>, MeriadocError> {
        let (project_filter, shell_name) = Self::parse_qualified_name(name);
        let mut matches: Vec<ResolvedShell<'a>> = Vec::new();

        for project in projects {
            if let Some(filter) = project_filter
                && Self::project_name(project) != filter
            {
                continue;
            }

            for spec_file in &project.specs {
                if let Some(shell_spec) = spec_file.spec.shells.get(shell_name) {
                    matches.push(ResolvedShell {
                        name: shell_name.to_string(),
                        spec: shell_spec,
                        spec_file,
                        project,
                    });
                }
            }
        }

        Self::resolve_single(matches, "shell", name)
    }

    fn resolve_single<T>(mut matches: Vec<T>, kind: &str, name: &str) -> Result<T, MeriadocError> {
        match matches.len() {
            0 => Err(MeriadocError::EntityNotFound {
                kind: kind.to_string(),
                name: name.to_string(),
            }),
            1 => Ok(matches.remove(0)),
            count => Err(MeriadocError::AmbiguousEntity {
                kind: kind.to_string(),
                name: name.to_string(),
                count,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    use crate::core::spec::{JobSpec, ShellSpec, SpecFile, TaskSpec};

    fn make_task(description: &str) -> TaskSpec {
        TaskSpec {
            description: Some(description.to_string()),
            cmds: vec!["echo test".to_string()],
            workdir: None,
            env: HashMap::new(),
            env_files: vec![],
            preconditions: vec![],
            on_failure: None,
            docs: None,
            agent: None,
        }
    }

    fn make_job(tasks: Vec<&str>) -> JobSpec {
        JobSpec {
            description: None,
            tasks: tasks.into_iter().map(|s| s.to_string()).collect(),
            env: HashMap::new(),
            env_files: vec![],
            on_failure: None,
        }
    }

    fn make_shell() -> ShellSpec {
        ShellSpec {
            description: None,
            workdir: None,
            env: HashMap::new(),
            env_files: vec![],
            init_cmds: vec![],
        }
    }

    fn make_project(name: &str, tasks: Vec<(&str, TaskSpec)>) -> Project {
        let mut spec = SpecFile {
            version: "v1".to_string(),
            tasks: HashMap::new(),
            jobs: HashMap::new(),
            shells: HashMap::new(),
        };

        for (task_name, task) in tasks {
            spec.tasks.insert(task_name.to_string(), task);
        }

        Project {
            root: PathBuf::from(format!("/projects/{}", name)),
            spec_files: vec![PathBuf::from(format!("/projects/{}/meriadoc.yaml", name))],
            specs: vec![LoadedSpec {
                path: PathBuf::from(format!("/projects/{}/meriadoc.yaml", name)),
                spec,
            }],
        }
    }

    fn make_project_with_jobs(name: &str, jobs: Vec<(&str, JobSpec)>) -> Project {
        let mut spec = SpecFile {
            version: "v1".to_string(),
            tasks: HashMap::new(),
            jobs: HashMap::new(),
            shells: HashMap::new(),
        };

        for (job_name, job) in jobs {
            spec.jobs.insert(job_name.to_string(), job);
        }

        Project {
            root: PathBuf::from(format!("/projects/{}", name)),
            spec_files: vec![PathBuf::from(format!("/projects/{}/meriadoc.yaml", name))],
            specs: vec![LoadedSpec {
                path: PathBuf::from(format!("/projects/{}/meriadoc.yaml", name)),
                spec,
            }],
        }
    }

    fn make_project_with_shells(name: &str, shells: Vec<(&str, ShellSpec)>) -> Project {
        let mut spec = SpecFile {
            version: "v1".to_string(),
            tasks: HashMap::new(),
            jobs: HashMap::new(),
            shells: HashMap::new(),
        };

        for (shell_name, shell) in shells {
            spec.shells.insert(shell_name.to_string(), shell);
        }

        Project {
            root: PathBuf::from(format!("/projects/{}", name)),
            spec_files: vec![PathBuf::from(format!("/projects/{}/meriadoc.yaml", name))],
            specs: vec![LoadedSpec {
                path: PathBuf::from(format!("/projects/{}/meriadoc.yaml", name)),
                spec,
            }],
        }
    }

    // ==================== parse_qualified_name tests ====================

    #[test]
    fn test_parse_qualified_name_unqualified() {
        let (project, entity) = EntityResolver::parse_qualified_name("mytask");
        assert_eq!(project, None);
        assert_eq!(entity, "mytask");
    }

    #[test]
    fn test_parse_qualified_name_qualified() {
        let (project, entity) = EntityResolver::parse_qualified_name("myproject:mytask");
        assert_eq!(project, Some("myproject"));
        assert_eq!(entity, "mytask");
    }

    #[test]
    fn test_parse_qualified_name_multiple_colons() {
        // Only first colon is used for splitting
        let (project, entity) = EntityResolver::parse_qualified_name("project:task:extra");
        assert_eq!(project, Some("project"));
        assert_eq!(entity, "task:extra");
    }

    // ==================== resolve_task tests ====================

    #[test]
    fn test_resolve_task_not_found_empty_projects() {
        let projects: Vec<Project> = vec![];
        let result = EntityResolver::resolve_task("mytask", &projects);

        assert!(matches!(
            result,
            Err(MeriadocError::EntityNotFound { kind, name })
            if kind == "task" && name == "mytask"
        ));
    }

    #[test]
    fn test_resolve_task_not_found() {
        let projects = vec![make_project("proj1", vec![("task1", make_task("desc"))])];
        let result = EntityResolver::resolve_task("nonexistent", &projects);

        assert!(matches!(
            result,
            Err(MeriadocError::EntityNotFound { kind, name })
            if kind == "task" && name == "nonexistent"
        ));
    }

    #[test]
    fn test_resolve_task_found_single() {
        let projects = vec![make_project(
            "proj1",
            vec![("mytask", make_task("my desc"))],
        )];
        let result = EntityResolver::resolve_task("mytask", &projects);

        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved.name, "mytask");
        assert_eq!(resolved.spec.description, Some("my desc".to_string()));
    }

    #[test]
    fn test_resolve_task_ambiguous() {
        let projects = vec![
            make_project("proj1", vec![("mytask", make_task("proj1 task"))]),
            make_project("proj2", vec![("mytask", make_task("proj2 task"))]),
        ];
        let result = EntityResolver::resolve_task("mytask", &projects);

        assert!(matches!(
            result,
            Err(MeriadocError::AmbiguousEntity { kind, name, count })
            if kind == "task" && name == "mytask" && count == 2
        ));
    }

    #[test]
    fn test_resolve_task_qualified_name() {
        let projects = vec![
            make_project("proj1", vec![("mytask", make_task("proj1 task"))]),
            make_project("proj2", vec![("mytask", make_task("proj2 task"))]),
        ];

        // Qualify to resolve ambiguity
        let result = EntityResolver::resolve_task("proj2:mytask", &projects);

        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved.spec.description, Some("proj2 task".to_string()));
    }

    #[test]
    fn test_resolve_task_qualified_not_found() {
        let projects = vec![make_project(
            "proj1",
            vec![("mytask", make_task("proj1 task"))],
        )];

        // Wrong project name
        let result = EntityResolver::resolve_task("proj2:mytask", &projects);

        assert!(matches!(
            result,
            Err(MeriadocError::EntityNotFound { kind, name })
            if kind == "task" && name == "proj2:mytask"
        ));
    }

    #[test]
    fn test_resolve_task_multiple_specs_same_project() {
        // A project can have multiple spec files; if both define the same task, it's ambiguous
        let mut project = make_project("proj1", vec![("mytask", make_task("spec1 task"))]);

        // Add another spec file with the same task
        let mut spec2 = SpecFile {
            version: "v1".to_string(),
            tasks: HashMap::new(),
            jobs: HashMap::new(),
            shells: HashMap::new(),
        };
        spec2
            .tasks
            .insert("mytask".to_string(), make_task("spec2 task"));
        project.specs.push(LoadedSpec {
            path: PathBuf::from("/projects/proj1/subdir/meriadoc.yaml"),
            spec: spec2,
        });

        let projects = vec![project];
        let result = EntityResolver::resolve_task("mytask", &projects);

        assert!(matches!(
            result,
            Err(MeriadocError::AmbiguousEntity { kind, name, count })
            if kind == "task" && name == "mytask" && count == 2
        ));
    }

    // ==================== resolve_job tests ====================

    #[test]
    fn test_resolve_job_found() {
        let projects = vec![make_project_with_jobs(
            "proj1",
            vec![("myjob", make_job(vec!["task1", "task2"]))],
        )];
        let result = EntityResolver::resolve_job("myjob", &projects);

        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved.name, "myjob");
        assert_eq!(resolved.spec.tasks, vec!["task1", "task2"]);
    }

    #[test]
    fn test_resolve_job_not_found() {
        let projects = vec![make_project_with_jobs(
            "proj1",
            vec![("myjob", make_job(vec!["task1"]))],
        )];
        let result = EntityResolver::resolve_job("nonexistent", &projects);

        assert!(matches!(
            result,
            Err(MeriadocError::EntityNotFound { kind, name })
            if kind == "job" && name == "nonexistent"
        ));
    }

    #[test]
    fn test_resolve_job_qualified() {
        let projects = vec![
            make_project_with_jobs("proj1", vec![("myjob", make_job(vec!["task1"]))]),
            make_project_with_jobs("proj2", vec![("myjob", make_job(vec!["task2"]))]),
        ];

        let result = EntityResolver::resolve_job("proj1:myjob", &projects);

        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved.spec.tasks, vec!["task1"]);
    }

    // ==================== resolve_shell tests ====================

    #[test]
    fn test_resolve_shell_found() {
        let projects = vec![make_project_with_shells(
            "proj1",
            vec![("dev", make_shell())],
        )];
        let result = EntityResolver::resolve_shell("dev", &projects);

        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved.name, "dev");
    }

    #[test]
    fn test_resolve_shell_not_found() {
        let projects = vec![make_project_with_shells(
            "proj1",
            vec![("dev", make_shell())],
        )];
        let result = EntityResolver::resolve_shell("prod", &projects);

        assert!(matches!(
            result,
            Err(MeriadocError::EntityNotFound { kind, name })
            if kind == "shell" && name == "prod"
        ));
    }

    #[test]
    fn test_resolve_shell_ambiguous() {
        let projects = vec![
            make_project_with_shells("proj1", vec![("dev", make_shell())]),
            make_project_with_shells("proj2", vec![("dev", make_shell())]),
        ];
        let result = EntityResolver::resolve_shell("dev", &projects);

        assert!(matches!(
            result,
            Err(MeriadocError::AmbiguousEntity { kind, name, count })
            if kind == "shell" && name == "dev" && count == 2
        ));
    }

    #[test]
    fn test_resolve_shell_qualified() {
        let projects = vec![
            make_project_with_shells("proj1", vec![("dev", make_shell())]),
            make_project_with_shells("proj2", vec![("dev", make_shell())]),
        ];

        let result = EntityResolver::resolve_shell("proj2:dev", &projects);
        assert!(result.is_ok());
    }
}
