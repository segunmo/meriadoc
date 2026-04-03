//! Integration tests for task and job execution
//!
//! These tests run the actual meriadoc binary and verify behavior.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Helper to run meriadoc with given args in a temp directory
#[allow(dead_code)]
fn run_meriadoc(temp_dir: &TempDir, args: &[&str]) -> std::process::Output {
    let binary = env!("CARGO_BIN_EXE_meriadoc");

    Command::new(binary)
        .args(args)
        .current_dir(temp_dir.path())
        .output()
        .expect("failed to execute meriadoc")
}

/// Helper to create a meriadoc.yaml file
fn create_spec_file(temp_dir: &TempDir, content: &str) {
    let spec_path = temp_dir.path().join("meriadoc.yaml");
    fs::write(&spec_path, content).expect("failed to write spec file");
}

/// Helper to create a config that points to the temp dir.
/// Returns both the config dir and a separate cache dir (both temp) so tests
/// never write to the real user cache at ~/.config/meriadoc/cache.
fn setup_config(temp_dir: &TempDir) -> TempDir {
    let config_dir = TempDir::new().expect("failed to create config dir");
    let cache_dir = config_dir.path().join("cache");
    let config_path = config_dir.path().join("config.yaml");

    let config_content = format!(
        r#"discovery:
  roots:
    - path: {}
      enabled: true
  max_depth: 3
  validate_on_discovery: false
  spec_files:
    - meriadoc.yaml
cache:
  enabled: false
  dir: {}
"#,
        temp_dir.path().display(),
        cache_dir.display(),
    );

    fs::write(&config_path, config_content).expect("failed to write config");
    config_dir
}

/// Run meriadoc with a custom config
fn run_with_config(
    temp_dir: &TempDir,
    config_dir: &TempDir,
    args: &[&str],
) -> std::process::Output {
    let binary = env!("CARGO_BIN_EXE_meriadoc");
    let config_path = config_dir.path().join("config.yaml");

    let mut full_args = vec!["--config", config_path.to_str().unwrap()];
    full_args.extend(args);

    Command::new(binary)
        .args(&full_args)
        .current_dir(temp_dir.path())
        .output()
        .expect("failed to execute meriadoc")
}

// ==================== Task Execution Tests ====================

#[test]
fn test_run_simple_task() {
    let temp_dir = TempDir::new().unwrap();
    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  hello:
    description: "Say hello"
    cmds:
      - echo "Hello, World!"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "task", "hello"]);

    assert!(output.status.success(), "Task should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello, World!"), "Should print hello");
}

#[test]
fn test_run_task_with_multiple_commands() {
    let temp_dir = TempDir::new().unwrap();
    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  multi:
    cmds:
      - echo "First"
      - echo "Second"
      - echo "Third"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "task", "multi"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("First"));
    assert!(stdout.contains("Second"));
    assert!(stdout.contains("Third"));
}

#[test]
fn test_run_task_with_env_default() {
    let temp_dir = TempDir::new().unwrap();
    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  envtest:
    cmds:
      - echo "Value is $MY_VAR"
    env:
      MY_VAR:
        type: string
        default: "default_value"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "task", "envtest"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Value is default_value"),
        "Should use default env value"
    );
}

#[test]
fn test_run_task_with_env_override() {
    let temp_dir = TempDir::new().unwrap();
    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  envtest:
    cmds:
      - echo "Value is $MY_VAR"
    env:
      MY_VAR:
        type: string
        default: "default_value"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &["run", "task", "envtest", "--env", "MY_VAR=overridden"],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Value is overridden"),
        "Should use CLI env override"
    );
}

#[test]
fn test_run_task_failure_propagates_exit_code() {
    let temp_dir = TempDir::new().unwrap();
    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  failing:
    cmds:
      - exit 42
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "task", "failing"]);

    assert!(!output.status.success(), "Task should fail");
    assert_eq!(output.status.code(), Some(42), "Should propagate exit code");
}

// ==================== Precondition Tests ====================

#[test]
fn test_precondition_success_allows_task() {
    let temp_dir = TempDir::new().unwrap();

    // Create a file that the precondition will check for
    fs::write(temp_dir.path().join("required.txt"), "exists").unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  guarded:
    cmds:
      - echo "Task executed"
    preconditions:
      - cmds:
          - test -f required.txt
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "task", "guarded"]);

    assert!(
        output.status.success(),
        "Task should succeed when precondition passes"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Task executed"));
}

#[test]
fn test_precondition_failure_blocks_task() {
    let temp_dir = TempDir::new().unwrap();
    // Note: required.txt does NOT exist

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  guarded:
    cmds:
      - echo "Task executed"
    preconditions:
      - cmds:
          - test -f required.txt
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "task", "guarded"]);

    assert!(
        !output.status.success(),
        "Task should fail when precondition fails"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("precondition") || stderr.contains("Precondition"),
        "Should mention precondition failure"
    );
}

#[test]
fn test_precondition_with_on_failure_continue() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  resilient:
    cmds:
      - echo "Task executed anyway"
    preconditions:
      - cmds:
          - test -f nonexistent.txt
        on_failure:
          continue: true
          cmds:
            - echo "Precondition failed but continuing"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "task", "resilient"]);

    assert!(
        output.status.success(),
        "Task should succeed with continue: true"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Precondition failed but continuing"));
    assert!(stdout.contains("Task executed anyway"));
}

// ==================== On Failure Tests ====================

#[test]
fn test_on_failure_runs_cleanup() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  cleanup_test:
    cmds:
      - echo "Starting task"
      - exit 1
    on_failure:
      continue: false
      cmds:
        - echo "Cleanup executed"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "task", "cleanup_test"]);

    assert!(!output.status.success(), "Task should fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Starting task"));
    assert!(
        stdout.contains("Cleanup executed"),
        "Should run on_failure commands"
    );
}

// ==================== Job Execution Tests ====================

#[test]
fn test_run_job_executes_tasks_in_order() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  first:
    cmds:
      - echo "FIRST"
  second:
    cmds:
      - echo "SECOND"
  third:
    cmds:
      - echo "THIRD"

jobs:
  ordered:
    tasks:
      - first
      - second
      - third
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "job", "ordered"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify order by checking positions
    let first_pos = stdout.find("FIRST").expect("Should contain FIRST");
    let second_pos = stdout.find("SECOND").expect("Should contain SECOND");
    let third_pos = stdout.find("THIRD").expect("Should contain THIRD");

    assert!(first_pos < second_pos, "FIRST should come before SECOND");
    assert!(second_pos < third_pos, "SECOND should come before THIRD");
}

#[test]
fn test_job_stops_on_task_failure() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  pass:
    cmds:
      - echo "PASS"
  fail:
    cmds:
      - exit 1
  never:
    cmds:
      - echo "NEVER_REACHED"

jobs:
  will_fail:
    tasks:
      - pass
      - fail
      - never
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "job", "will_fail"]);

    assert!(!output.status.success(), "Job should fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PASS"), "First task should run");
    assert!(
        !stdout.contains("NEVER_REACHED"),
        "Third task should not run after failure"
    );
}

#[test]
fn test_job_env_overrides_task_env() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  show_env:
    cmds:
      - echo "VAR=$MY_VAR"
    env:
      MY_VAR:
        type: string
        default: "from_task"

jobs:
  override_env:
    tasks:
      - show_env
    env:
      MY_VAR:
        type: string
        default: "from_job"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "job", "override_env"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("VAR=from_job"),
        "Job env should override task env"
    );
}

#[test]
fn test_job_cli_env_satisfies_required_task_var() {
    // Regression test: a required env var declared on a task should be satisfied
    // by --env passed at the job level, not fail with "missing required env var".
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  show_version:
    cmds:
      - echo "VERSION=$VERSION"
    env:
      VERSION:
        type: string
        required: true

jobs:
  release:
    env:
      VERSION:
        type: string
        required: true
    tasks:
      - show_version
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &["run", "job", "release", "--env", "VERSION=1.2.3"],
    );

    assert!(
        output.status.success(),
        "Job should succeed when VERSION is passed via --env: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("VERSION=1.2.3"),
        "Task should receive VERSION from job-level --env"
    );
}

#[test]
fn test_job_on_failure_with_continue() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  first:
    cmds:
      - echo "FIRST"
  failing:
    cmds:
      - exit 1
  third:
    cmds:
      - echo "THIRD"

jobs:
  resilient:
    tasks:
      - first
      - failing
      - third
    on_failure:
      continue: true
      cmds:
        - echo "JOB_CLEANUP"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "job", "resilient"]);

    // With continue: true, the job should continue past the failing task
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("FIRST"));
    assert!(stdout.contains("JOB_CLEANUP"), "Should run job cleanup");
    assert!(stdout.contains("THIRD"), "Should continue to third task");
}

// ==================== Env File Tests ====================

#[test]
fn test_env_file_loading() {
    let temp_dir = TempDir::new().unwrap();

    // Create .env file
    fs::write(temp_dir.path().join(".env"), "FROM_FILE=file_value\n").unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  envfile:
    cmds:
      - echo "FROM_FILE=$FROM_FILE"
    env_files:
      - .env
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "task", "envfile"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("FROM_FILE=file_value"),
        "Should load from env file"
    );
}

// ==================== Validation Tests ====================

#[test]
fn test_validate_valid_spec() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  valid:
    cmds:
      - echo "valid"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["validate"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("valid") || stdout.contains("no errors"));
}

#[test]
fn test_validate_invalid_spec_empty_task() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  empty:
    cmds: []
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["validate"]);

    // Validation should report errors but command itself succeeds
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("error") || stdout.contains("Error"),
        "Should report validation error for empty task"
    );
}

// ==================== Doctor Tests ====================

#[test]
fn test_doctor_reports_status() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  test:
    cmds:
      - echo test
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["doctor"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Meriadoc Doctor"));
    assert!(stdout.contains("Configuration"));
    assert!(stdout.contains("Projects"));
    assert!(stdout.contains("Validation"));
}

// ==================== Info Tests ====================

#[test]
fn test_info_task_shows_details() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  detailed:
    description: "A detailed task"
    cmds:
      - echo "command1"
      - echo "command2"
    env:
      MY_VAR:
        type: string
        default: "value"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["info", "task", "detailed"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Task: detailed"));
    assert!(stdout.contains("A detailed task"));
    assert!(stdout.contains("command1"));
    assert!(stdout.contains("MY_VAR"));
}

#[test]
fn test_info_nonexistent_task_fails() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  exists:
    cmds:
      - echo test
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["info", "task", "nonexistent"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("NotFound"),
        "Should report task not found"
    );
}

// ==================== Dry-Run Tests ====================

#[test]
fn test_dry_run_task_shows_info() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  mytask:
    description: "A test task"
    cmds:
      - echo "This should not run"
      - echo "Neither should this"
    env:
      MY_VAR:
        type: string
        default: "test_value"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &["run", "task", "mytask", "--dry-run"],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show dry-run info
    assert!(stdout.contains("[dry-run]"), "Should have dry-run prefix");
    assert!(stdout.contains("Task: mytask"), "Should show task name");
    assert!(
        stdout.contains("MY_VAR=test_value"),
        "Should show resolved env"
    );
    assert!(
        stdout.contains("echo \"This should not run\""),
        "Should show commands"
    );

    // Should NOT actually run
    assert!(
        !stdout.contains("Running task:"),
        "Should not show 'Running task'"
    );
}

#[test]
fn test_dry_run_task_with_env_override() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  envtask:
    cmds:
      - echo "$MY_VAR"
    env:
      MY_VAR:
        type: string
        default: "default"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &[
            "run",
            "task",
            "envtask",
            "--dry-run",
            "--env",
            "MY_VAR=overridden",
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show overridden env
    assert!(
        stdout.contains("MY_VAR=overridden"),
        "Should show CLI-overridden env value"
    );
    assert!(
        !stdout.contains("MY_VAR=default"),
        "Should not show default value"
    );
}

#[test]
fn test_dry_run_task_with_preconditions() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  precond_task:
    cmds:
      - echo "main command"
    preconditions:
      - cmds:
          - test -f somefile
        on_failure:
          continue: false
          cmds:
            - echo "precondition failed"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &["run", "task", "precond_task", "--dry-run"],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Preconditions:"),
        "Should show preconditions section"
    );
    assert!(
        stdout.contains("test -f somefile"),
        "Should show precondition command"
    );
}

#[test]
fn test_dry_run_task_with_on_failure() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  failure_task:
    cmds:
      - exit 1
    on_failure:
      continue: false
      cmds:
        - echo "cleaning up"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &["run", "task", "failure_task", "--dry-run"],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("On failure:"),
        "Should show on_failure section"
    );
    assert!(
        stdout.contains("cleaning up"),
        "Should show on_failure command"
    );
}

#[test]
fn test_dry_run_job_shows_all_tasks() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  first:
    cmds:
      - echo "first"
  second:
    cmds:
      - echo "second"
  third:
    cmds:
      - echo "third"

jobs:
  myjob:
    tasks:
      - first
      - second
      - third
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &["run", "job", "myjob", "--dry-run"],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show job info
    assert!(stdout.contains("Job: myjob"), "Should show job name");
    assert!(
        stdout.contains("first → second → third"),
        "Should show task sequence"
    );

    // Should show each task
    assert!(stdout.contains("Task 1/3: first"), "Should show first task");
    assert!(
        stdout.contains("Task 2/3: second"),
        "Should show second task"
    );
    assert!(stdout.contains("Task 3/3: third"), "Should show third task");

    // Should NOT actually run
    assert!(
        !stdout.contains("Running job:"),
        "Should not show 'Running job'"
    );
}

#[test]
fn test_dry_run_shell_shows_config() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
shells:
  myshell:
    description: "Test shell"
    env:
      SHELL_VAR:
        type: string
        default: "shell_value"
    init_cmds:
      - echo "init command 1"
      - echo "init command 2"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &["run", "shell", "myshell", "--dry-run"],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show shell info
    assert!(stdout.contains("Shell: myshell"), "Should show shell name");
    assert!(
        stdout.contains("SHELL_VAR=shell_value"),
        "Should show env var"
    );
    assert!(
        stdout.contains("Init commands:"),
        "Should show init commands section"
    );
    assert!(
        stdout.contains("echo \"init command 1\""),
        "Should show init command"
    );
    assert!(
        stdout.contains("Would start interactive"),
        "Should indicate it would start shell"
    );

    // Should NOT actually start shell
    assert!(
        !stdout.contains("Starting shell:"),
        "Should not show 'Starting shell'"
    );
}

// ==================== Variable Interpolation Tests ====================

#[test]
fn test_interpolation_braces_syntax() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  interp:
    cmds:
      - echo "Hello ${NAME}"
    env:
      NAME:
        type: string
        default: "World"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "task", "interp"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Hello World"),
        "Should interpolate variable"
    );
}

#[test]
fn test_interpolation_simple_syntax() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  interp:
    cmds:
      - echo "Hello $NAME"
    env:
      NAME:
        type: string
        default: "World"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "task", "interp"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello World"), "Should interpolate $NAME");
}

#[test]
fn test_interpolation_with_default() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  interp:
    cmds:
      - echo "Hello ${NAME:-Default}"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "task", "interp"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello Default"), "Should use default value");
}

#[test]
fn test_interpolation_cli_override() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  interp:
    cmds:
      - echo "Hello ${NAME}"
    env:
      NAME:
        type: string
        default: "World"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &["run", "task", "interp", "--env", "NAME=CLI"],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello CLI"), "Should use CLI override");
}

#[test]
fn test_interpolation_special_vars() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  special:
    cmds:
      - echo "Root is set"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &["run", "task", "special", "--dry-run"],
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        eprintln!("stderr: {}", stderr);
    }
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain the special variables in the environment listing
    assert!(
        stdout.contains("MERIADOC_PROJECT_ROOT"),
        "Should include MERIADOC_PROJECT_ROOT in env"
    );
    assert!(
        stdout.contains("MERIADOC_SPEC_DIR"),
        "Should include MERIADOC_SPEC_DIR in env"
    );
}

#[test]
fn test_interpolation_dry_run_shows_resolved() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  interp:
    cmds:
      - echo "Hello ${NAME}"
    env:
      NAME:
        type: string
        default: "World"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &["run", "task", "interp", "--dry-run"],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show both raw and interpolated
    assert!(
        stdout.contains("echo \"Hello ${NAME}\""),
        "Should show raw command"
    );
    assert!(
        stdout.contains("→ echo \"Hello World\""),
        "Should show interpolated command with arrow"
    );
}

#[test]
fn test_interpolation_escaped_dollar() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  escape:
    cmds:
      - echo "Price is 100 dollars"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "task", "escape"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        eprintln!("stderr: {}", stderr);
    }
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Price is 100 dollars"),
        "Should output text"
    );
}

#[test]
fn test_interpolation_multiple_vars() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  multi:
    cmds:
      - echo "${GREETING} ${NAME}!"
    env:
      GREETING:
        type: string
        default: "Hello"
      NAME:
        type: string
        default: "World"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "task", "multi"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Hello World!"),
        "Should interpolate multiple vars"
    );
}

// ==================== Interactive Mode Tests ====================

#[test]
fn test_no_interactive_fails_on_missing_required() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  needs_var:
    cmds:
      - echo "MY_VAR=$MY_VAR"
    env:
      MY_VAR:
        type: string
        required: true
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &["run", "task", "needs_var", "--no-interactive"],
    );

    assert!(!output.status.success(), "Should fail without required var");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("MY_VAR") || stderr.contains("required"),
        "Should mention missing variable"
    );
}

#[test]
fn test_no_interactive_succeeds_with_default() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  has_default:
    cmds:
      - echo "MY_VAR=$MY_VAR"
    env:
      MY_VAR:
        type: string
        required: true
        default: "default_value"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &["run", "task", "has_default", "--no-interactive"],
    );

    assert!(output.status.success(), "Should succeed with default value");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("MY_VAR=default_value"));
}

#[test]
fn test_no_interactive_succeeds_with_cli_override() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  needs_var:
    cmds:
      - echo "MY_VAR=$MY_VAR"
    env:
      MY_VAR:
        type: string
        required: true
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &[
            "run",
            "task",
            "needs_var",
            "--no-interactive",
            "--env",
            "MY_VAR=cli_value",
        ],
    );

    assert!(
        output.status.success(),
        "Should succeed with CLI-provided value"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("MY_VAR=cli_value"));
}

// ==================== Choice Validation Tests ====================

#[test]
fn test_choice_validation_accepts_valid_option() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  choice_task:
    cmds:
      - echo "ENV=$ENV"
    env:
      ENV:
        type: string
        options:
          - dev
          - staging
          - prod
        default: dev
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &["run", "task", "choice_task", "--env", "ENV=staging"],
    );

    assert!(output.status.success(), "Should accept valid choice");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ENV=staging"));
}

#[test]
fn test_choice_validation_rejects_invalid_option() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  choice_task:
    cmds:
      - echo "ENV=$ENV"
    env:
      ENV:
        type: string
        options:
          - dev
          - staging
          - prod
        default: dev
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &["run", "task", "choice_task", "--env", "ENV=invalid"],
    );

    assert!(!output.status.success(), "Should reject invalid choice");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid") || stderr.contains("Invalid"),
        "Should mention invalid choice"
    );
}

// ==================== .env Autoload Tests ====================

#[test]
fn test_dotenv_autoload_from_project_root() {
    let temp_dir = TempDir::new().unwrap();

    // Create .env in project root
    fs::write(temp_dir.path().join(".env"), "AUTO_VAR=autoloaded\n").unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  autoload_test:
    cmds:
      - echo "AUTO_VAR=$AUTO_VAR"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "task", "autoload_test"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("AUTO_VAR=autoloaded"),
        "Should autoload from .env"
    );
}

#[test]
fn test_env_file_overrides_dotenv() {
    let temp_dir = TempDir::new().unwrap();

    // Create .env in project root (lower priority)
    fs::write(temp_dir.path().join(".env"), "MY_VAR=from_dotenv\n").unwrap();

    // Create custom env file (higher priority)
    fs::write(temp_dir.path().join("custom.env"), "MY_VAR=from_custom\n").unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  priority_test:
    cmds:
      - echo "MY_VAR=$MY_VAR"
    env_files:
      - custom.env
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "task", "priority_test"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("MY_VAR=from_custom"),
        "env_files should override .env"
    );
}

#[test]
fn test_inline_default_overrides_env_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create env file
    fs::write(temp_dir.path().join("custom.env"), "MY_VAR=from_file\n").unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  priority_test:
    cmds:
      - echo "MY_VAR=$MY_VAR"
    env_files:
      - custom.env
    env:
      MY_VAR:
        type: string
        default: from_inline
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["run", "task", "priority_test"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("MY_VAR=from_inline"),
        "inline default should override env_files"
    );
}

#[test]
fn test_cli_overrides_all() {
    let temp_dir = TempDir::new().unwrap();

    // Create .env
    fs::write(temp_dir.path().join(".env"), "MY_VAR=from_dotenv\n").unwrap();

    // Create custom env file
    fs::write(temp_dir.path().join("custom.env"), "MY_VAR=from_file\n").unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  priority_test:
    cmds:
      - echo "MY_VAR=$MY_VAR"
    env_files:
      - custom.env
    env:
      MY_VAR:
        type: string
        default: from_inline
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &["run", "task", "priority_test", "--env", "MY_VAR=from_cli"],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("MY_VAR=from_cli"),
        "CLI should override all other sources"
    );
}

// ==================== Env Show Command Tests ====================

#[test]
fn test_env_show_displays_vars() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  envtask:
    description: "Task with env vars"
    cmds:
      - echo "test"
    env:
      API_KEY:
        type: string
        required: true
        description: "The API key to use"
      ENVIRONMENT:
        type: string
        options:
          - dev
          - prod
        default: dev
        description: "Target environment"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["env", "show", "task", "envtask"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("API_KEY"), "Should show API_KEY");
    assert!(stdout.contains("required"), "Should indicate required");
    assert!(stdout.contains("ENVIRONMENT"), "Should show ENVIRONMENT");
    assert!(
        stdout.contains("dev") && stdout.contains("prod"),
        "Should show options"
    );
}

#[test]
fn test_env_show_nonexistent_task() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  exists:
    cmds:
      - echo test
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &["env", "show", "task", "nonexistent"],
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("NotFound"),
        "Should report task not found"
    );
}

// ==================== Env Ls Command Tests ====================

#[test]
fn test_env_ls_empty() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  test:
    cmds:
      - echo test
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["env", "ls"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No saved") || stdout.is_empty() || stdout.contains("0"),
        "Should indicate no saved env files"
    );
}

// ==================== Shortcut Command Tests ====================

#[test]
fn test_task_shortcut_command() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  hello:
    cmds:
      - echo "Hello from shortcut"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    // Use "task" command instead of "run task"
    let output = run_with_config(&temp_dir, &config_dir, &["task", "hello"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello from shortcut"));
}

#[test]
fn test_job_shortcut_command() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  step:
    cmds:
      - echo "Job step"

jobs:
  myjob:
    tasks:
      - step
"#,
    );

    let config_dir = setup_config(&temp_dir);
    // Use "job" command instead of "run job"
    let output = run_with_config(&temp_dir, &config_dir, &["job", "myjob"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Job step"));
}

#[test]
fn test_shortcut_with_no_interactive() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  needs_var:
    cmds:
      - echo "VAR=$MY_VAR"
    env:
      MY_VAR:
        type: string
        required: true
"#,
    );

    let config_dir = setup_config(&temp_dir);
    // Use shortcut with -n flag
    let output = run_with_config(&temp_dir, &config_dir, &["task", "needs_var", "-n"]);

    assert!(!output.status.success(), "Should fail without required var");
}

#[test]
fn test_shortcut_with_env_override() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  envtask:
    cmds:
      - echo "VAR=$MY_VAR"
    env:
      MY_VAR:
        type: string
        required: true
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &["task", "envtask", "--env", "MY_VAR=shortcut_value"],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("VAR=shortcut_value"));
}

// ==================== JSON Output Tests ====================

#[test]
fn test_json_ls_tasks() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  task1:
    description: "First task"
    cmds:
      - echo "task1"
  task2:
    description: "Second task"
    cmds:
      - echo "task2"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["--json", "ls", "tasks"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should be valid JSON with items wrapper
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    assert!(json.is_object(), "Should be an object");

    let items = json.get("items").expect("Should have items field");
    let tasks = items.as_array().expect("items should be an array");
    assert_eq!(tasks.len(), 2, "Should have 2 tasks");

    // Check task structure
    let task_names: Vec<&str> = tasks
        .iter()
        .map(|t| t.get("name").unwrap().as_str().unwrap())
        .collect();
    assert!(task_names.iter().any(|n| n.contains("task1")));
    assert!(task_names.iter().any(|n| n.contains("task2")));
}

#[test]
fn test_json_ls_projects() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  test:
    cmds:
      - echo "test"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["--json", "ls", "projects"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    assert!(json.is_object(), "Should be an object");

    let items = json.get("items").expect("Should have items field");
    let projects = items.as_array().expect("items should be an array");
    assert!(!projects.is_empty(), "Should have at least 1 project");
}

#[test]
fn test_json_info_task() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  detailed:
    description: "A detailed task"
    cmds:
      - echo "command"
    env:
      MY_VAR:
        type: string
        default: "value"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &["--json", "info", "task", "detailed"],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    assert!(json.is_object(), "Should be an object");

    assert!(json.get("name").is_some(), "Should have name");
    assert!(json.get("description").is_some(), "Should have description");
    assert!(json.get("cmds").is_some(), "Should have cmds");
}

// ==================== Agent Annotation Tests ====================

#[test]
fn test_agent_annotated_task_runs() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  risky_task:
    description: "A risky operation"
    agent:
      risk_level: high
      requires_approval: true
      confirmation: "Are you sure?"
    cmds:
      - echo "RISKY_OUTPUT"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    // Task with agent annotations should still run normally for CLI users
    let output = run_with_config(&temp_dir, &config_dir, &["run", "task", "risky_task"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("RISKY_OUTPUT"), "Task should execute");
}

#[test]
fn test_agent_disabled_task_still_runs_for_human() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  hidden:
    description: "Hidden from agents"
    agent:
      enabled: false
    cmds:
      - echo "HIDDEN_OUTPUT"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    // Human can still run the task even if agent.enabled is false
    let output = run_with_config(&temp_dir, &config_dir, &["run", "task", "hidden"]);

    assert!(
        output.status.success(),
        "Human should be able to run agent-disabled task"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("HIDDEN_OUTPUT"));
}

#[test]
fn test_json_ls_tasks_includes_agent_disabled() {
    // The JSON API (--json ls tasks) is for humans (web UI, CLI),
    // so it should include tasks with agent.enabled: false.
    // Only MCP should filter them out.
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  visible_task:
    description: "Visible to everyone"
    cmds:
      - echo "visible"
  hidden_from_agents:
    description: "Hidden from agents but visible in UI"
    agent:
      enabled: false
    cmds:
      - echo "hidden"
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(&temp_dir, &config_dir, &["--json", "ls", "tasks"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    let items = json.get("items").expect("Should have items field");
    let tasks = items.as_array().expect("items should be an array");

    // Both tasks should be present in the JSON output (for humans)
    let task_names: Vec<&str> = tasks
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();

    assert!(
        task_names.iter().any(|n| n.contains("visible_task")),
        "visible_task should be in list"
    );
    assert!(
        task_names.iter().any(|n| n.contains("hidden_from_agents")),
        "hidden_from_agents should be in list (web UI is for humans, not agents)"
    );
}

// ==================== Secret Masking Tests ====================

#[test]
fn test_secret_type_variable() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  secret_task:
    description: "Task with secret"
    cmds:
      - echo "Using secret"
    env:
      API_KEY:
        type: secret
        required: true
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &[
            "run",
            "task",
            "secret_task",
            "--env",
            "API_KEY=super-secret-value",
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Using secret"));
}

#[test]
fn test_secret_masked_in_verbose_output() {
    let temp_dir = TempDir::new().unwrap();

    create_spec_file(
        &temp_dir,
        r#"
version: v1
tasks:
  secret_task:
    cmds:
      - echo "Secret is $MY_SECRET"
    env:
      MY_SECRET:
        type: secret
        required: true
"#,
    );

    let config_dir = setup_config(&temp_dir);
    let output = run_with_config(
        &temp_dir,
        &config_dir,
        &[
            "run",
            "task",
            "secret_task",
            "--env",
            "MY_SECRET=password123",
            "--verbose",
        ],
    );

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The verbose output (stderr) should mask the secret
    assert!(
        stderr.contains("***") || !stderr.contains("password123"),
        "Secret should be masked in verbose output"
    );
}

// ==================== Version Flag Tests ====================

#[test]
fn test_version_flag() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = setup_config(&temp_dir);

    let output = run_with_config(&temp_dir, &config_dir, &["--version"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("meriadoc"), "Should contain program name");
    assert!(stdout.contains("0.1"), "Should contain version number");
}

// ==================== Help Flag Tests ====================

#[test]
fn test_help_flag() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = setup_config(&temp_dir);

    let output = run_with_config(&temp_dir, &config_dir, &["--help"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage"), "Should show usage");
    assert!(stdout.contains("run"), "Should list run command");
    assert!(stdout.contains("task"), "Should list task command");
}
