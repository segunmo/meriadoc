# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-03-15

### Fixed

- Release workflow: fixed SHA256 checksum extraction for Homebrew formula (`grep -h` to suppress filename prefix)
- Release workflow: replaced deprecated `macos-13` runner with `macos-14` for `x86_64-apple-darwin` cross-compilation
- Install script: detect musl libc on Linux (Alpine) and select the correct binary variant

[0.1.1]: https://github.com/segunmo/meriadoc/releases/tag/v0.1.1

## [0.1.0] - 2025-02-11

### Added

- **Tasks**: Run sequential shell commands with environment variables, working directory, and preconditions
- **Jobs**: Compose multiple tasks into workflows with shared environment
- **Shells**: Start interactive shell sessions with pre-configured environments and custom prompts
- **Project Discovery**: Automatically find projects across configured directories
- **Validation**: Check spec files before execution with comprehensive error messages
- **Validation Caching**: Skip re-validation of unchanged files using SHA-256 hashing
- **Entity Resolution**: Support qualified names (`project:task`) to disambiguate between projects
- **Environment Variables**: Priority-based resolution (CLI > inline > env_files)
- **Choice Validation**: Runtime validation of `choice` type env vars against allowed options
- **Interactive Prompting**: Prompt for missing required variables in TTY mode
- **Saved Environments**: Store prompted values in `~/.config/meriadoc/env/<project>/<task>.env`
- **Variable Interpolation**: `${VAR}`, `$VAR`, and `${VAR:-default}` syntax in commands
- **Special Variables**: `${MERIADOC_PROJECT_ROOT}` and `${MERIADOC_SPEC_DIR}` automatically available
- **Dry-Run Mode**: Preview what would happen without executing (`--dry-run`)
- **Preconditions**: Check conditions before task execution with on_failure handlers
- **On-Failure Handlers**: Run cleanup commands when tasks fail
- **CLI Commands**:
  - `meriadoc run task/job/shell <name>` - Execute tasks, jobs, or shells
  - `meriadoc task <name>` / `meriadoc t <name>` - Shortcut for run task
  - `meriadoc job <name>` / `meriadoc j <name>` - Shortcut for run job
  - `meriadoc shell <name>` / `meriadoc s <name>` - Shortcut for run shell
  - `-n` / `--no-interactive` - Never prompt, fail on missing vars
  - `-i` / `--interactive` - Always prompt for variables
  - `meriadoc ls projects/tasks/jobs/shells` - List entities
  - `meriadoc info task/job/shell/project <name>` - Show detailed information
  - `meriadoc validate` - Validate all spec files
  - `meriadoc config add/rm/ls` - Manage discovery roots
  - `meriadoc cache ls/clear` - Manage validation cache
  - `meriadoc env show task/job/shell <name>` - Show environment variable requirements
  - `meriadoc doctor` - Diagnose common issues

### Spec File Format

- Support for `meriadoc.yaml`, `meriadoc.yml`, `merry.yaml`, `merry.yml`
- Version `v1` spec format with tasks, jobs, and shells sections
- Environment variable specs with type hints, defaults, options, and required flags
- Preconditions with on_failure policies
- Job-level and task-level on_failure handlers

[0.1.0]: https://github.com/segunmo/meriadoc/releases/tag/v0.1.0
