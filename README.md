# Meriadoc

A task runner designed for both humans and AI agents. Define tasks, jobs, and shells in YAML spec files with typed environment variables, risk annotations, and multiple interfaces.

Meriadoc consolidates operational knowledge that typically scatters across READMEs, CI files, and tribal memory into explicit, discoverable, and validated spec files.

## Features

- **Tasks**: Sequential shell commands with environment variables, working directory, and preconditions
- **Jobs**: Compose multiple tasks into workflows
- **Shells**: Interactive sessions with pre-configured environments
- **Discovery**: Automatically find projects across configured directories
- **Validation**: Check specs before execution
- **Multiple Interfaces**: CLI, Web UI, and MCP (Model Context Protocol) for AI agents

---

## For Humans

### Quick Start

**1. Create a spec file** (`meriadoc.yaml` in your project):

```yaml
version: v1

tasks:
  build:
    description: "Build the project"
    cmds:
      - cargo build --release

  test:
    description: "Run tests"
    cmds:
      - cargo test

  deploy:
    description: "Deploy to an environment"
    agent:
      risk_level: high
      confirmation: "This will deploy to production. Continue?"
    cmds:
      - ./deploy.sh
    env:
      ENVIRONMENT:
        type: choice
        options: [dev, staging, prod]
        default: dev

jobs:
  ci:
    description: "Full CI pipeline"
    tasks:
      - build
      - test

shells:
  dev:
    description: "Development environment"
    env:
      RUST_LOG:
        type: string
        default: debug
```

**2. Add the project:**

```bash
meriadoc config add /path/to/your/project
```

**3. Run tasks:**

```bash
meriadoc ls tasks              # List available tasks
meriadoc run task build        # Run a task
meriadoc run job ci            # Run a job
meriadoc run shell dev         # Start a shell
```

### Web UI

Start the HTTP server for a browser-based interface:

```bash
meriadoc server
# Opens at http://localhost:8420
```

The web UI provides:
- Projects panel with task/job/shell counts
- Tasks grouped by project with collapsible sections
- Task info modal showing commands, env vars, and metadata
- Run modal with typed input fields for environment variables
- Dry-run option to preview commands without executing
- Real-time output console

### CLI Reference

```bash
# Discovery & Listing
meriadoc ls                       # List projects
meriadoc ls tasks                 # List all tasks
meriadoc ls jobs                  # List all jobs
meriadoc ls shells                # List all shells

# Running (with shortcuts)
meriadoc run task <name>          # or: meriadoc task <name> / meriadoc t <name>
meriadoc run job <name>           # or: meriadoc job <name> / meriadoc j <name>
meriadoc run shell <name>         # or: meriadoc shell <name> / meriadoc s <name>

# Options
meriadoc run task <name> --env KEY=VALUE    # Override env var
meriadoc run task <name> --dry-run          # Preview without executing
meriadoc run task <name> --prompt-all       # Review all env vars

# Information
meriadoc info task <name>         # Show task details
meriadoc env show task <name>     # Show env vars for a task

# Configuration
meriadoc config add <path>        # Add a project directory
meriadoc config rm <path>         # Remove a project directory
meriadoc config ls                # List configured directories

# Validation
meriadoc validate                 # Validate all specs
meriadoc doctor                   # Diagnose common issues

# JSON output (for scripts)
meriadoc --json ls tasks
meriadoc --json info task deploy
```

### Environment Variable Types

```yaml
env:
  MY_STRING:
    type: string
    default: "hello"

  MY_NUMBER:
    type: number          # Decimal numbers (3.14)
    default: "3.14"

  MY_INTEGER:
    type: integer         # Whole numbers only
    default: "42"

  MY_BOOL:
    type: boolean         # true/false
    default: "true"

  MY_CHOICE:
    type: choice          # Constrained options
    options: [dev, staging, prod]
    default: dev

  MY_FILE:
    type: filepath        # File path
    default: "/tmp/file.txt"

  MY_SECRET:
    type: secret          # Masked in output
    required: true
```

---

## For Agents

Meriadoc provides a secure execution boundary for AI coding agents with structured discovery, risk annotations, and multiple integration options.

### MCP Server (Model Context Protocol)

Expose tasks as MCP tools for AI agents like Claude:

```bash
meriadoc serve
```

This starts a JSON-RPC server over stdio that implements the MCP protocol. AI agents can:
- Discover available tasks with `tools/list`
- Execute tasks with `tools/call`
- Receive structured output with success/failure status

**Claude Desktop Integration** (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "meriadoc": {
      "command": "/path/to/meriadoc",
      "args": ["serve"]
    }
  }
}
```

### HTTP API

The HTTP server provides REST endpoints for programmatic access:

```bash
meriadoc server --port 8420
```

**Endpoints:**

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/projects` | List all projects |
| GET | `/api/tasks` | List all tasks with metadata |
| GET | `/api/tasks/:name/info` | Get detailed task info |
| POST | `/api/tasks/:name/run` | Execute a task |
| POST | `/mcp` | MCP JSON-RPC endpoint |

**Example - Run a task:**

```bash
curl -X POST http://localhost:8420/api/tasks/myproject:build/run \
  -H "Content-Type: application/json" \
  -d '{"env": [["DEBUG", "true"]], "dry_run": false}'
```

### Agent Annotations

Mark tasks with risk levels and control agent visibility:

```yaml
tasks:
  safe-task:
    description: "Low-risk read-only operation"
    agent:
      risk_level: low      # low, medium, high, critical
    cmds:
      - cat status.txt

  deploy-prod:
    description: "Deploy to production"
    agent:
      risk_level: critical
      requires_approval: true
      confirmation: "This will deploy to production. Are you sure?"
    cmds:
      - ./deploy.sh prod

  internal-task:
    description: "Not exposed to agents"
    agent:
      enabled: false       # Hidden from agent discovery
    cmds:
      - ./internal-script.sh
```

**Risk Levels:**

| Level | Description | Behavior |
|-------|-------------|----------|
| `low` | Safe, read-only operations | Auto-approved |
| `medium` | Reversible changes | Auto-approved (logged) |
| `high` | Significant changes | Requires approval |
| `critical` | Destructive/irreversible | Requires explicit approval |

### Typed Environment Variables

Agents receive schema information for each task's environment:

```json
{
  "env_vars": [
    {
      "name": "ENVIRONMENT",
      "type": "choice",
      "required": true,
      "default": "dev",
      "options": ["dev", "staging", "prod"]
    },
    {
      "name": "API_KEY",
      "type": "secret",
      "required": true,
      "default": null,
      "options": []
    }
  ]
}
```

This enables agents to:
- Understand required vs optional parameters
- Validate values against allowed options
- Generate appropriate UI or prompts

### Why Meriadoc for Agents?

Traditional task runners (Make, Just, Taskfile) lack:

| Feature | Make/Just | Meriadoc |
|---------|-----------|----------|
| Structured metadata | No | Yes (descriptions, types) |
| Risk annotations | No | Yes (risk levels, approval gates) |
| Agent visibility control | No | Yes (`agent.enabled: false`) |
| Typed parameters | No | Yes (string, choice, secret, etc.) |
| Programmatic output | Limited | Yes (JSON, MCP) |
| Web UI | No | Yes |

Meriadoc provides capability-based security where agents can only execute predefined tasks with clear contracts and human oversight for risky operations.

---

## Spec File Reference

### Tasks

```yaml
tasks:
  mytask:
    description: "What this task does"
    cmds:
      - echo "First command"
      - echo "Second command"
    workdir: src                    # Optional, relative to project root
    env:
      MY_VAR:
        type: string
        default: "value"
    env_files:
      - .env                        # Load from dotenv files
    preconditions:
      - cmds:
          - test -f required.txt
        on_failure:
          continue: false
          cmds:
            - echo "Missing required.txt"
    on_failure:
      continue: false
      cmds:
        - echo "Task failed, cleaning up"
    agent:
      risk_level: low               # Agent metadata
      requires_approval: false
```

### Jobs

```yaml
jobs:
  myjob:
    description: "Run multiple tasks"
    tasks:
      - task1
      - task2
      - task3
    env:
      SHARED_VAR:
        type: string
        default: "shared"           # Overrides task-level env
    on_failure:
      continue: true                # Continue to next task on failure
```

### Shells

```yaml
shells:
  dev:
    description: "Development shell"
    workdir: src
    env:
      DEBUG:
        type: string
        default: "true"
    init_cmds:
      - source .env
      - echo "Shell ready"
```

### Variable Interpolation

```yaml
tasks:
  build:
    cmds:
      - echo "Building ${VERSION}"
      - echo "Project: ${MERIADOC_PROJECT_ROOT}"
    env:
      VERSION:
        type: string
        default: "1.0.0"
```

**Supported syntax:**
- `${VAR}` or `$VAR` - Variable value
- `${VAR:-default}` - Default if unset
- `$$` - Literal `$`

**Built-in variables:**
- `${MERIADOC_PROJECT_ROOT}` - Project root path
- `${MERIADOC_SPEC_DIR}` - Spec file directory

---

## Examples

<!-- TODO: Add screenshots and videos -->

### Web UI

*Screenshot: Projects and tasks panel*

*Screenshot: Task info modal*

*Screenshot: Run modal with typed inputs*

### CLI

*Demo: Running a task with dry-run*

*Demo: Interactive environment prompts*

### MCP Integration

*Demo: Claude using Meriadoc tasks*

---

## Configuration

Global configuration is stored at `~/.config/meriadoc/config.yaml`:

```yaml
discovery:
  roots:
    - path: /home/user/projects
      enabled: true
  max_depth: 3
  spec_files:
    - meriadoc.yaml
    - meriadoc.yml
    - merry.yaml
    - merry.yml

cache:
  enabled: true
  dir: .meriadoc/cache
```

---

## License

MIT
