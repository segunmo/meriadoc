# Meriadoc – Architecture Documentation

## 1. Overview

**Meriadoc** is a local-first developer productivity tool designed to manage and execute ad‑hoc scripts, internal tools, and contextual development workflows. It targets both:

* **Solo developers** maintaining a personal script library
* **Teams** publishing internal tools and workflows via shared repositories

Meriadoc emphasizes:

* Portability across machines
* Explicit environments and contexts
* Interactive and non-interactive execution
* Minimal friction for onboarding

The tool is invoked via the CLI as:

```bash
meriadoc
# or
merry
```

and is callable from any directory.

---

## 2. Installation & Local Layout

### 2.1 Installation

Installation will be handled via a system package manager (e.g. Homebrew). The installer ensures:

* The `meriadoc` (and optional `merry`) binary is available on `$PATH`
* A user-local directory is created

```text
/Users/Alice/.meriadoc/
```

This directory is **owned by the tool**, not by projects.

---

### 2.2 Meriadoc Home Directory Structure

```text
~/.meriadoc/
├── config.yaml          # Global user configuration
├── state.yaml           # Tool-managed runtime state (optional)
├── projects.yaml        # Registered project roots
├── cache/               # Optional caches (repos, metadata)
└── logs/
```

---

## 3. Configuration Model

### 3.1 Global Configuration (`config.yaml`)

The global configuration defines **where Meriadoc should look for projects** and user preferences.

Example:

```yaml
version: "0.1"
projects:
  - ~/work/team-infra
  - ~/work/image-tools
  - ~/scripts
ui:
  theme: dark
language: en
```

Key points:

* Project directories are **external** to Meriadoc
* Meriadoc never requires projects to live under `~/.meriadoc/`
* Each project directory is scanned for Meriadoc spec files

---

## 4. Project Model

A **project** is any directory registered in the global config that contains Meriadoc specification files.

### 4.1 Project Root

The project root is the directory containing one or more of:

* `tasks.yaml`
* `jobs.yaml`
* `shells.yaml`
* `meriadoc.yaml` (optional umbrella file)

All relative paths in specs are resolved **from this root**.

---

## 5. Execution Semantics

### 5.1 Default Working Directory Resolution

Meriadoc applies different defaults depending on what is executed.

#### Tasks & Jobs

* **Default working directory**: the directory containing the YAML file that defines the task or job
* If `workdir` is specified: resolved **relative to the project root**

#### Shells

* **Default working directory**: the directory from which `meriadoc` was invoked
* If `workdir` is specified: resolved **relative to the project root**

This distinction supports both:

* Repository-defined workflows
* Ad-hoc interactive usage

---

## 6. Specification Files (v0.1)

All specs are versioned. Additive changes are expected; breaking changes require a version bump.

---

### 6.1 Task Specification

A **Task** is the smallest execution unit.

```yaml
version: "0.1"
task:
  name: string
  description?: string
  cmds: [string]
  workdir?: string
  env?: { string: EnvVar }
  env_files?: [string]
  preconditions?: [Condition]
  on_failure?: FailurePolicy
  docs?: string
```

**Semantics**:

* `cmds` execute sequentially
* Environment variables are resolved before execution
* `workdir` is relative to project root

---

### 6.2 Job Specification

A **Job** is a composition of tasks.

```yaml
version: "0.1"
job:
  name: string
  description?: string
  tasks: [string]
  env?: { string: EnvVar }
  env_files?: [string]
  on_failure?: FailurePolicy
```

**Semantics**:

* Tasks run sequentially in the order listed
* Task names are resolved within the same project

---

### 6.3 Shell Specification

A **Shell** creates an interactive session with a resolved context.

```yaml
version: "0.1"
shell:
  name: string
  description?: string
  workdir?: string
  env?: { string: EnvVar }
  env_files?: [string]
  init_cmds?: [string]
```

**Semantics**:

* An interactive shell is spawned
* Environment variables are injected
* `init_cmds` run before handing control to the user
* User may execute arbitrary commands until exit

---

### 6.4 Environment Variable Specification

```yaml
EnvVar:
  type: string
  default?: string
  options?: [string]
  required?: boolean
```

**Purpose**:

* Typed, user-selectable configuration
* Enables validation and safe prompting

---

### 6.5 Condition Specification

```yaml
Condition:
  cmds: [string]
  on_failure?: FailurePolicy
```

Conditions must succeed for execution to proceed.

---

### 6.6 Failure Policy

```yaml
FailurePolicy:
  continue: boolean
  cmds?: [string]
```

Controls error handling behavior.

---

## 7. Architectural Principles

* **Local-first**: no daemon, no cloud dependency
* **Explicit roots**: all relative paths resolve from a project root
* **Portability**: specs never reference absolute user paths
* **Additive evolution**: schemas grow without breaking old projects
* **Separation of concerns**:

  * Specs define intent
  * Executor handles processes
  * UI orchestrates interaction

---

## 8. Future Extensions (Non-v1)

Explicitly out of scope for v0.1:

* Parallel execution
* Task dependencies / DAGs
* Includes / imports
* Templating
* Remote execution

Schemas are designed to allow **additive** introduction of these features.

---

## 9. Summary

Meriadoc treats scripts as **products**, not one-off commands. By anchoring execution to project roots and separating global configuration from project specs, it enables teams and individuals to share reliable, portable tooling without sacrificing local control.
