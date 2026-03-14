//! MCP method handlers - business logic for MCP requests.

use serde_json::{Value, json};

use crate::app::App;
use crate::core::resolver::EntityResolver;
use crate::core::spec::RiskLevel;
use crate::mcp::types::*;

/// MCP method handlers
pub struct McpHandlers;

impl McpHandlers {
    /// Handle initialize request
    pub fn initialize() -> InitializeResult {
        InitializeResult {
            protocol_version: "2025-06-18".to_string(),
            capabilities: ServerCapabilities {
                tools: ToolsCapability {
                    list_changed: false,
                },
            },
            server_info: ServerInfo {
                name: "meriadoc".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        }
    }

    /// Handle tools/list request - returns all available tools
    pub fn list_tools(app: &App) -> ToolsListResult {
        let mut tools = Vec::new();

        // Meta tool: list all tasks
        tools.push(Tool {
            name: "meriadoc_list_tasks".to_string(),
            title: Some("List Tasks".to_string()),
            description: "List all available Meriadoc tasks".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        });

        // Meta tool: run any task by name
        tools.push(Tool {
            name: "meriadoc_run_task".to_string(),
            title: Some("Run Task".to_string()),
            description: "Execute a Meriadoc task by name".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "Task name (e.g., 'project:taskname' or just 'taskname')"
                    },
                    "env": {
                        "type": "object",
                        "description": "Environment variable overrides",
                        "additionalProperties": { "type": "string" }
                    },
                    "dry_run": {
                        "type": "boolean",
                        "description": "Preview without executing",
                        "default": false
                    }
                },
                "required": ["task"]
            }),
        });

        // Add individual tasks as direct tools (filtered by agent.enabled)
        app.for_each_agent_task(|info| {
            let qualified_name = format!("{}:{}", info.project_name, info.task_name);

            // Build description with optional confirmation message
            let mut description = info
                .task
                .description
                .clone()
                .unwrap_or_else(|| format!("Run task {}", qualified_name));

            if let Some(agent) = &info.task.agent
                && let Some(confirmation) = &agent.confirmation
            {
                description.push_str(&format!("\n\nConfirmation required: {}", confirmation));
            }

            // Get risk level (default to low)
            let risk_level = info
                .task
                .agent
                .as_ref()
                .map(|a| a.risk_level)
                .unwrap_or(RiskLevel::Low);

            // Determine if approval is required
            let requires_approval = info
                .task
                .agent
                .as_ref()
                .map(|a| {
                    a.requires_approval
                        || matches!(a.risk_level, RiskLevel::High | RiskLevel::Critical)
                })
                .unwrap_or(false);

            // Build input schema from task's env vars
            let mut properties = serde_json::Map::new();
            let mut required_vars = Vec::new();

            for (env_name, env_spec) in &info.task.env {
                let mut prop = serde_json::Map::new();
                prop.insert("type".to_string(), json!("string"));

                if let Some(default) = &env_spec.default {
                    prop.insert("default".to_string(), json!(default));
                }

                if !env_spec.options.is_empty() {
                    prop.insert("enum".to_string(), json!(env_spec.options));
                }

                properties.insert(env_name.clone(), Value::Object(prop));

                // Track required vars (required with no default)
                if env_spec.required && env_spec.default.is_none() {
                    required_vars.push(env_name.clone());
                }
            }

            // Use underscore-safe tool name
            let tool_name = format!("task_{}_{}", info.project_name, info.task_name);

            tools.push(Tool {
                name: tool_name,
                title: Some(info.task_name.to_string()),
                description,
                input_schema: json!({
                    "type": "object",
                    "properties": properties,
                    "required": required_vars,
                    "x-risk-level": risk_level.as_str(),
                    "x-requires-approval": requires_approval
                }),
            });
        });

        ToolsListResult { tools }
    }

    /// Handle tools/call request
    pub fn call_tool(app: &mut App, params: &ToolCallParams) -> ToolCallResult {
        match params.name.as_str() {
            "meriadoc_list_tasks" => Self::handle_list_tasks(app),
            "meriadoc_run_task" => Self::handle_run_task(app, params),
            name if name.starts_with("task_") => Self::handle_direct_task(app, params),
            _ => ToolCallResult::error(format!("Unknown tool: {}", params.name)),
        }
    }

    /// List all available tasks (respects agent.enabled filtering)
    fn handle_list_tasks(app: &App) -> ToolCallResult {
        let mut output = String::new();

        app.for_each_agent_task(|info| {
            let desc = info.task.description.as_deref().unwrap_or("");
            let risk = info
                .task
                .agent
                .as_ref()
                .map(|a| a.risk_level.as_str())
                .unwrap_or("low");

            output.push_str(&format!(
                "{}:{} [{}] - {}\n",
                info.project_name, info.task_name, risk, desc
            ));
        });

        if output.is_empty() {
            output = "No tasks found.".to_string();
        }

        ToolCallResult::success(output)
    }

    /// Run a task by name
    fn handle_run_task(app: &mut App, params: &ToolCallParams) -> ToolCallResult {
        // Extract task name
        let task_name = match params.arguments.get("task") {
            Some(Value::String(s)) => s.clone(),
            _ => return ToolCallResult::error("Missing required parameter: task"),
        };

        let dry_run = params
            .arguments
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Extract env overrides
        let env_overrides: Vec<(String, String)> = params
            .arguments
            .get("env")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        Self::execute_task(app, &task_name, &env_overrides, dry_run)
    }

    /// Run a task directly via task_project_name tool
    fn handle_direct_task(app: &mut App, params: &ToolCallParams) -> ToolCallResult {
        // Parse task name from tool name: task_project_taskname -> project:taskname
        let tool_suffix = params.name.strip_prefix("task_").unwrap_or(&params.name);

        // Find first underscore to split project and task name
        let task_name = if let Some(idx) = tool_suffix.find('_') {
            format!("{}:{}", &tool_suffix[..idx], &tool_suffix[idx + 1..])
        } else {
            tool_suffix.to_string()
        };

        // All arguments are env overrides for direct task tools
        let env_overrides: Vec<(String, String)> = params
            .arguments
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect();

        Self::execute_task(app, &task_name, &env_overrides, false)
    }

    /// Check if a task requires approval before execution.
    /// Returns the confirmation message if approval is needed, None otherwise.
    fn check_approval_required(app: &App, task_name: &str) -> Option<String> {
        // Resolve the task
        let resolved = EntityResolver::resolve_task(task_name, &app.projects).ok()?;

        if let Some(agent) = &resolved.spec.agent {
            // Check explicit requires_approval flag first
            if agent.requires_approval {
                return Some(
                    agent
                        .confirmation
                        .clone()
                        .unwrap_or_else(|| "This task requires approval".to_string()),
                );
            }

            // Check risk level - high and critical require approval
            if matches!(agent.risk_level, RiskLevel::High | RiskLevel::Critical) {
                return Some(agent.confirmation.clone().unwrap_or_else(|| {
                    format!(
                        "This is a {} risk operation. Please confirm.",
                        agent.risk_level.as_str()
                    )
                }));
            }
        }

        None
    }

    /// Execute a task and return result
    fn execute_task(
        app: &mut App,
        task_name: &str,
        env_overrides: &[(String, String)],
        dry_run: bool,
    ) -> ToolCallResult {
        // Check approval BEFORE execution
        if let Some(confirmation_msg) = Self::check_approval_required(app, task_name) {
            return ToolCallResult::error(format!(
                "APPROVAL REQUIRED: {}\n\n\
                 To proceed, confirm with the human and retry this tool call.",
                confirmation_msg
            ));
        }

        match crate::app::commands::run_task_for_mcp(app, task_name, env_overrides, dry_run) {
            Ok(output) => ToolCallResult::success(output),
            Err(e) => ToolCallResult::error(format!("Error: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::spec::{AgentSpec, RiskLevel, SpecFile, TaskSpec};
    use crate::repo::project::LoadedSpec;
    use crate::repo::{Project, ValidationCache};
    use std::collections::HashMap;
    use std::path::PathBuf;

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

    fn make_task_with_agent(description: &str, enabled: bool, risk: RiskLevel) -> TaskSpec {
        TaskSpec {
            description: Some(description.to_string()),
            cmds: vec!["echo test".to_string()],
            workdir: None,
            env: HashMap::new(),
            env_files: vec![],
            preconditions: vec![],
            on_failure: None,
            docs: None,
            agent: Some(AgentSpec {
                enabled,
                risk_level: risk,
                confirmation: None,
                requires_approval: false,
            }),
        }
    }

    fn make_task_with_approval(
        description: &str,
        risk: RiskLevel,
        confirmation: Option<&str>,
        requires_approval: bool,
    ) -> TaskSpec {
        TaskSpec {
            description: Some(description.to_string()),
            cmds: vec!["echo test".to_string()],
            workdir: None,
            env: HashMap::new(),
            env_files: vec![],
            preconditions: vec![],
            on_failure: None,
            docs: None,
            agent: Some(AgentSpec {
                enabled: true,
                risk_level: risk,
                confirmation: confirmation.map(|s| s.to_string()),
                requires_approval,
            }),
        }
    }

    fn make_spec_file(tasks: HashMap<String, TaskSpec>) -> SpecFile {
        SpecFile {
            version: "0.1".to_string(),
            tasks,
            jobs: HashMap::new(),
            shells: HashMap::new(),
        }
    }

    fn make_test_app(tasks: HashMap<String, TaskSpec>) -> App {
        let spec_file = make_spec_file(tasks);
        // Project name is derived from root directory name
        let project = Project {
            root: PathBuf::from("/testproject"),
            spec_files: vec![PathBuf::from("/testproject/meriadoc.yaml")],
            specs: vec![LoadedSpec {
                path: PathBuf::from("/testproject/meriadoc.yaml"),
                spec: spec_file,
            }],
        };
        App {
            projects: vec![project],
            config: crate::config::MeriadocConfig::default(),
            cache: ValidationCache::new(),
        }
    }

    #[test]
    fn test_initialize_returns_correct_version() {
        let result = McpHandlers::initialize();
        assert_eq!(result.protocol_version, "2025-06-18");
        assert_eq!(result.server_info.name, "meriadoc");
    }

    #[test]
    fn test_list_tools_includes_meta_tools() {
        let app = make_test_app(HashMap::new());
        let result = McpHandlers::list_tools(&app);

        let tool_names: Vec<&str> = result.tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"meriadoc_list_tasks"));
        assert!(tool_names.contains(&"meriadoc_run_task"));
    }

    #[test]
    fn test_list_tools_includes_tasks() {
        let mut tasks = HashMap::new();
        tasks.insert("mytask".to_string(), make_task("My task"));

        let app = make_test_app(tasks);
        let result = McpHandlers::list_tools(&app);

        let tool_names: Vec<&str> = result.tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"task_testproject_mytask"));
    }

    #[test]
    fn test_list_tools_filters_disabled_agents() {
        let mut tasks = HashMap::new();
        tasks.insert(
            "enabled_task".to_string(),
            make_task_with_agent("Enabled", true, RiskLevel::Low),
        );
        tasks.insert(
            "disabled_task".to_string(),
            make_task_with_agent("Disabled", false, RiskLevel::Low),
        );

        let app = make_test_app(tasks);
        let result = McpHandlers::list_tools(&app);

        let tool_names: Vec<&str> = result.tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"task_testproject_enabled_task"));
        assert!(!tool_names.contains(&"task_testproject_disabled_task"));
    }

    #[test]
    fn test_list_tools_includes_risk_level() {
        let mut tasks = HashMap::new();
        tasks.insert(
            "risky_task".to_string(),
            make_task_with_agent("Risky", true, RiskLevel::High),
        );

        let app = make_test_app(tasks);
        let result = McpHandlers::list_tools(&app);

        let tool = result
            .tools
            .iter()
            .find(|t| t.name == "task_testproject_risky_task")
            .expect("Tool not found");

        let risk_level = tool.input_schema.get("x-risk-level").unwrap();
        assert_eq!(risk_level, "high");
    }

    #[test]
    fn test_list_tools_default_risk_level_is_low() {
        let mut tasks = HashMap::new();
        tasks.insert("normal_task".to_string(), make_task("Normal task"));

        let app = make_test_app(tasks);
        let result = McpHandlers::list_tools(&app);

        let tool = result
            .tools
            .iter()
            .find(|t| t.name == "task_testproject_normal_task")
            .expect("Tool not found");

        let risk_level = tool.input_schema.get("x-risk-level").unwrap();
        assert_eq!(risk_level, "low");
    }

    #[test]
    fn test_handle_list_tasks_filters_disabled() {
        let mut tasks = HashMap::new();
        tasks.insert(
            "visible".to_string(),
            make_task_with_agent("Visible", true, RiskLevel::Low),
        );
        tasks.insert(
            "hidden".to_string(),
            make_task_with_agent("Hidden", false, RiskLevel::Low),
        );

        let app = make_test_app(tasks);
        let result = McpHandlers::handle_list_tasks(&app);

        assert!(!result.is_error);
        let text = &result.content[0].text;
        assert!(text.contains("visible"));
        assert!(!text.contains("hidden"));
    }

    #[test]
    fn test_handle_list_tasks_shows_risk_level() {
        let mut tasks = HashMap::new();
        tasks.insert(
            "critical_task".to_string(),
            make_task_with_agent("Critical", true, RiskLevel::Critical),
        );

        let app = make_test_app(tasks);
        let result = McpHandlers::handle_list_tasks(&app);

        let text = &result.content[0].text;
        assert!(text.contains("[critical]"));
    }

    #[test]
    fn test_call_tool_unknown_returns_error() {
        let mut app = make_test_app(HashMap::new());
        let params = ToolCallParams {
            name: "unknown_tool".to_string(),
            arguments: HashMap::new(),
        };

        let result = McpHandlers::call_tool(&mut app, &params);

        assert!(result.is_error);
        assert!(result.content[0].text.contains("Unknown tool"));
    }

    #[test]
    fn test_high_risk_requires_approval() {
        let mut tasks = HashMap::new();
        tasks.insert(
            "deploy".to_string(),
            make_task_with_approval("Deploy to prod", RiskLevel::High, Some("Deploy?"), false),
        );

        let app = make_test_app(tasks);
        let result = McpHandlers::check_approval_required(&app, "deploy");

        assert!(result.is_some());
        assert_eq!(result.unwrap(), "Deploy?");
    }

    #[test]
    fn test_critical_risk_requires_approval() {
        let mut tasks = HashMap::new();
        tasks.insert(
            "delete".to_string(),
            make_task_with_approval("Delete data", RiskLevel::Critical, None, false),
        );

        let app = make_test_app(tasks);
        let result = McpHandlers::check_approval_required(&app, "delete");

        assert!(result.is_some());
        assert!(result.unwrap().contains("critical"));
    }

    #[test]
    fn test_low_risk_no_approval() {
        let mut tasks = HashMap::new();
        tasks.insert(
            "safe".to_string(),
            make_task_with_approval("Safe task", RiskLevel::Low, None, false),
        );

        let app = make_test_app(tasks);
        let result = McpHandlers::check_approval_required(&app, "safe");

        assert!(result.is_none());
    }

    #[test]
    fn test_medium_risk_no_approval() {
        let mut tasks = HashMap::new();
        tasks.insert(
            "medium".to_string(),
            make_task_with_approval("Medium task", RiskLevel::Medium, None, false),
        );

        let app = make_test_app(tasks);
        let result = McpHandlers::check_approval_required(&app, "medium");

        assert!(result.is_none());
    }

    #[test]
    fn test_explicit_requires_approval() {
        let mut tasks = HashMap::new();
        tasks.insert(
            "notify".to_string(),
            make_task_with_approval(
                "Send notification",
                RiskLevel::Low,
                Some("Send to all users?"),
                true, // Explicit requires_approval
            ),
        );

        let app = make_test_app(tasks);
        let result = McpHandlers::check_approval_required(&app, "notify");

        assert!(result.is_some());
        assert_eq!(result.unwrap(), "Send to all users?");
    }

    #[test]
    fn test_list_tools_shows_requires_approval() {
        let mut tasks = HashMap::new();
        tasks.insert(
            "risky".to_string(),
            make_task_with_approval("Risky task", RiskLevel::High, None, false),
        );
        tasks.insert(
            "safe".to_string(),
            make_task_with_approval("Safe task", RiskLevel::Low, None, false),
        );

        let app = make_test_app(tasks);
        let result = McpHandlers::list_tools(&app);

        let risky_tool = result
            .tools
            .iter()
            .find(|t| t.name == "task_testproject_risky")
            .expect("Risky tool not found");
        let safe_tool = result
            .tools
            .iter()
            .find(|t| t.name == "task_testproject_safe")
            .expect("Safe tool not found");

        assert_eq!(
            risky_tool.input_schema.get("x-requires-approval").unwrap(),
            true
        );
        assert_eq!(
            safe_tool.input_schema.get("x-requires-approval").unwrap(),
            false
        );
    }
}
