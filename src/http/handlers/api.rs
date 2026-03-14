//! REST API handlers.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::sse::{Event, Sse},
};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;

use crate::app::commands::run_task_for_mcp;
use crate::core::resolver::EntityResolver;
use crate::core::spec::{RiskLevel, VarType};

use super::super::state::AppState;

/// Environment variable specification for API responses.
#[derive(Serialize)]
pub struct EnvVarInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub var_type: VarType,
    pub required: bool,
    pub default: Option<String>,
    pub options: Vec<String>,
}

/// Task information for API responses.
#[derive(Serialize)]
pub struct TaskInfo {
    pub name: String,
    pub project: String,
    pub description: Option<String>,
    pub risk_level: String,
    pub requires_approval: bool,
    pub env_vars: Vec<EnvVarInfo>,
}

/// Response for task list endpoint.
#[derive(Serialize)]
pub struct TaskListResponse {
    pub tasks: Vec<TaskInfo>,
}

/// List all available tasks.
/// Note: This shows ALL tasks including those with agent.enabled=false,
/// since the web UI is for humans, not AI agents.
pub async fn list_tasks(State(state): State<AppState>) -> Json<TaskListResponse> {
    let app = state.app.read();
    let mut tasks = vec![];

    app.for_each_task(|info| {
        let (risk_level, requires_approval) = info
            .task
            .agent
            .as_ref()
            .map(|a| {
                (
                    a.risk_level.as_str().to_string(),
                    a.requires_approval
                        || matches!(a.risk_level, RiskLevel::High | RiskLevel::Critical),
                )
            })
            .unwrap_or(("low".to_string(), false));

        let env_vars: Vec<EnvVarInfo> = info
            .task
            .env
            .iter()
            .map(|(name, spec)| EnvVarInfo {
                name: name.clone(),
                var_type: spec.var_type,
                required: spec.required,
                default: spec.default.clone(),
                options: spec.options.clone(),
            })
            .collect();

        tasks.push(TaskInfo {
            name: info.task_name.to_string(),
            project: info.project_name.to_string(),
            description: info.task.description.clone(),
            risk_level,
            requires_approval,
            env_vars,
        });
    });

    Json(TaskListResponse { tasks })
}

/// Project information for API responses.
#[derive(Serialize)]
pub struct ProjectInfo {
    pub name: String,
    pub root: String,
    pub task_count: usize,
    pub job_count: usize,
    pub shell_count: usize,
}

/// Response for project list endpoint.
#[derive(Serialize)]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectInfo>,
}

/// List all projects.
pub async fn list_projects(State(state): State<AppState>) -> Json<ProjectListResponse> {
    let app = state.app.read();
    let projects = app
        .projects
        .iter()
        .map(|project| {
            let name = EntityResolver::project_name(project);
            let mut task_count = 0;
            let mut job_count = 0;
            let mut shell_count = 0;

            for spec in &project.specs {
                task_count += spec.spec.tasks.len();
                job_count += spec.spec.jobs.len();
                shell_count += spec.spec.shells.len();
            }

            ProjectInfo {
                name: name.to_string(),
                root: project.root.display().to_string(),
                task_count,
                job_count,
                shell_count,
            }
        })
        .collect();

    Json(ProjectListResponse { projects })
}

/// Request body for running a task.
#[derive(Deserialize, Default)]
pub struct RunTaskRequest {
    #[serde(default)]
    pub env: Vec<(String, String)>,
    #[serde(default)]
    pub dry_run: bool,
}

/// Response for task execution.
#[derive(Serialize)]
pub struct RunTaskResponse {
    pub success: bool,
    pub output: String,
    pub exit_code: i32,
}

/// Run a task by name.
pub async fn run_task(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<RunTaskRequest>,
) -> Json<RunTaskResponse> {
    let mut app = state.app.write();

    match run_task_for_mcp(&mut app, &name, &payload.env, payload.dry_run) {
        Ok(output) => Json(RunTaskResponse {
            success: true,
            output,
            exit_code: 0,
        }),
        Err(e) => Json(RunTaskResponse {
            success: false,
            output: format!("Error: {}", e),
            exit_code: 1,
        }),
    }
}

/// Run a task with SSE streaming output.
pub async fn run_task_stream(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut app = state.app.write();

    let result = run_task_for_mcp(&mut app, &name, &[], false);

    let mut events: Vec<Result<Event, Infallible>> = match result {
        Ok(output) => output
            .lines()
            .map(|line| Ok(Event::default().data(line)))
            .collect(),
        Err(e) => {
            vec![Ok(Event::default().data(format!("Error: {}", e)))]
        }
    };

    // Add completion event
    events.push(Ok(Event::default().event("done").data("")));

    Sse::new(stream::iter(events))
}

/// Detailed task information for the info endpoint.
#[derive(Serialize)]
pub struct TaskDetailInfo {
    pub name: String,
    pub project: String,
    pub description: Option<String>,
    pub workdir: Option<String>,
    pub cmds: Vec<String>,
    pub env_vars: Vec<EnvVarInfo>,
    pub env_files: Vec<String>,
    pub has_preconditions: bool,
    pub has_on_failure: bool,
    pub risk_level: String,
    pub requires_approval: bool,
}

/// Get detailed information about a task.
pub async fn task_info(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<TaskDetailInfo>, (StatusCode, String)> {
    let app = state.app.read();

    let resolved = EntityResolver::resolve_task(&name, &app.projects)
        .map_err(|e| (StatusCode::NOT_FOUND, format!("Task not found: {}", e)))?;

    let (risk_level, requires_approval) = resolved
        .spec
        .agent
        .as_ref()
        .map(|a| {
            (
                a.risk_level.as_str().to_string(),
                a.requires_approval
                    || matches!(a.risk_level, RiskLevel::High | RiskLevel::Critical),
            )
        })
        .unwrap_or(("low".to_string(), false));

    let env_vars: Vec<EnvVarInfo> = resolved
        .spec
        .env
        .iter()
        .map(|(name, spec)| EnvVarInfo {
            name: name.clone(),
            var_type: spec.var_type,
            required: spec.required,
            default: spec.default.clone(),
            options: spec.options.clone(),
        })
        .collect();

    Ok(Json(TaskDetailInfo {
        name: name.clone(),
        project: EntityResolver::project_name(resolved.project).to_string(),
        description: resolved.spec.description.clone(),
        workdir: resolved.spec.workdir.clone(),
        cmds: resolved.spec.cmds.clone(),
        env_vars,
        env_files: resolved.spec.env_files.clone(),
        has_preconditions: !resolved.spec.preconditions.is_empty(),
        has_on_failure: resolved.spec.on_failure.is_some(),
        risk_level,
        requires_approval,
    }))
}
