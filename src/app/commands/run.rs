use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::app::App;
use crate::cli::{CliPrompter, EditableVar, EnvPrompter, InteractiveMode, RunKind, RunOptions};
use crate::config::spec::CacheConfig;
use crate::core::execution::{
    CommandRunner, EnvResolver, ExecutionOptions, InteractiveShell, MissingVar, ResolvedEnv,
    WorkdirMode,
};
use crate::core::resolver::EntityResolver;
use crate::core::spec::{ConditionSpec, EnvVarSpec};
use crate::core::validation::MeriadocError;
use crate::repo::{project_cache_dir, SavedEnvStore, ValidationCache};

/// Result of environment resolution, including any issues found.
struct EnvResult {
    env: ResolvedEnv,
    missing: Vec<MissingVar>,
    invalid_choices: Vec<InvalidChoice>,
}

/// A variable with an invalid choice value.
struct InvalidChoice {
    var: String,
    value: String,
    options: Vec<String>,
}

/// Find required variables that are missing from the resolved env.
fn find_missing_required_vars(
    inline_env: &HashMap<String, EnvVarSpec>,
    resolved: &ResolvedEnv,
) -> Vec<MissingVar> {
    let mut missing = Vec::new();
    let mut sorted_keys: Vec<_> = inline_env.keys().collect();
    sorted_keys.sort();

    for key in sorted_keys {
        let spec = &inline_env[key];
        if spec.required && !resolved.contains_key(key) {
            missing.push(MissingVar {
                name: key.clone(),
                spec: spec.clone(),
            });
        }
    }
    missing
}

/// Extract names of secret variables from env spec.
fn extract_secrets(inline_env: &HashMap<String, EnvVarSpec>) -> HashSet<String> {
    inline_env
        .iter()
        .filter(|(_, spec)| spec.var_type.is_sensitive())
        .map(|(name, _)| name.clone())
        .collect()
}

/// Find variables with values that don't match their allowed options.
fn find_invalid_choices(
    inline_env: &HashMap<String, EnvVarSpec>,
    resolved: &ResolvedEnv,
) -> Vec<InvalidChoice> {
    let mut invalid = Vec::new();
    let mut sorted_keys: Vec<_> = inline_env.keys().collect();
    sorted_keys.sort();

    for key in sorted_keys {
        let spec = &inline_env[key];
        if !spec.options.is_empty()
            && let Some(value) = resolved.get(key)
            && !spec.options.contains(value)
        {
            invalid.push(InvalidChoice {
                var: key.clone(),
                value: value.clone(),
                options: spec.options.clone(),
            });
        }
    }
    invalid
}

/// Result of running a task, including captured output.
pub struct TaskRunResult {
    /// Exit code (0 = success)
    pub exit_code: i32,
    /// Combined stdout/stderr output (empty if capture_output was false)
    pub output: String,
}

/// Handle run command - dispatches to task/job/shell runners.
pub fn handle_run(
    kind: RunKind,
    name: String,
    options: RunOptions,
    app: &mut App,
) -> Result<(), MeriadocError> {
    let mode = InteractiveMode::from_options(&options);
    let exec_options = ExecutionOptions {
        verbose: options.verbose,
        timeout: options.timeout,
        capture_output: false, // CLI mode: output goes to terminal
        ..Default::default()
    };

    let exit_code = match kind {
        RunKind::Task => RunActions::run_task(app, &name, &options, mode, &exec_options)?.exit_code,
        RunKind::Job => RunActions::run_job(app, &name, &options, mode, &exec_options)?,
        RunKind::Shell => RunActions::run_shell(app, &name, &options, mode)?,
    };

    if exit_code != 0 {
        std::process::exit(exit_code);
    }

    Ok(())
}

struct RunActions;

impl RunActions {
    /// Add special Meriadoc variables to the environment
    fn add_special_vars(env: &mut ResolvedEnv, project_root: &Path, spec_file_dir: &Path) {
        env.insert(
            "MERIADOC_PROJECT_ROOT".to_string(),
            project_root.to_string_lossy().to_string(),
        );
        env.insert(
            "MERIADOC_SPEC_DIR".to_string(),
            spec_file_dir.to_string_lossy().to_string(),
        );
    }

    /// Resolve environment with interactive prompting support.
    ///
    /// Priority order (highest to lowest):
    /// 1. CLI --env
    /// 2. CLI --env-file
    /// 3. Saved env (~/.config/meriadoc/env/<project>/<entity>.env)
    /// 4. Inline env: defaults from spec
    /// 5. env_files: from spec
    /// 6. .env in project root (standard fallback)
    ///
    /// When `dry_run` is true, skips prompting and returns issues in the result
    /// instead of erroring.
    #[allow(clippy::too_many_arguments)]
    fn resolve_env_interactive(
        inline_env: &HashMap<String, EnvVarSpec>,
        env_files: &[String],
        options: &RunOptions,
        project_root: &Path,
        project_name: &str,
        entity_name: &str,
        mode: InteractiveMode,
        config_dir: &Path,
    ) -> Result<EnvResult, MeriadocError> {
        // Build env in priority order (lowest to highest)
        let mut resolved = ResolvedEnv::new();

        // 1. Load .env from project root (lowest priority)
        let dotenv_path = project_root.join(".env");
        if dotenv_path.exists() {
            match EnvResolver::load_env_file(&dotenv_path) {
                Ok(dotenv) => {
                    for (key, value) in dotenv {
                        resolved.insert(key, value);
                    }
                }
                Err(e) => {
                    eprintln!("Warning: failed to load {}: {}", dotenv_path.display(), e);
                }
            }
        }

        // 2. Load env_files from spec
        for env_file in env_files {
            let path = project_root.join(env_file);
            if path.exists() {
                match EnvResolver::load_env_file(&path) {
                    Ok(file_env) => {
                        for (key, value) in file_env {
                            resolved.insert(key, value);
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: failed to load {}: {}", path.display(), e);
                    }
                }
            }
        }

        // 3. Apply inline env defaults
        for (key, spec) in inline_env {
            if let Some(default) = &spec.default {
                resolved.insert(key.clone(), default.clone());
            }
        }

        // 4. Load saved env (overrides inline defaults)
        let saved_store = SavedEnvStore::new(config_dir);
        let saved_env = saved_store.load(project_name, entity_name)?;
        for (key, value) in saved_env {
            resolved.insert(key, value);
        }

        // 5. Load CLI --env-file (higher priority than saved env)
        if let Some(cli_file) = &options.env_file {
            if cli_file.exists() {
                match EnvResolver::load_env_file(cli_file) {
                    Ok(file_env) => {
                        for (key, value) in file_env {
                            resolved.insert(key, value);
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: failed to load --env-file {}: {}",
                            cli_file.display(),
                            e
                        );
                    }
                }
            } else {
                eprintln!("Warning: --env-file not found: {}", cli_file.display());
            }
        }

        // 6. Apply CLI --env (highest priority)
        for (key, value) in &options.env {
            resolved.insert(key.clone(), value.clone());
        }

        // Check for missing required vars and invalid choices
        let mut missing = find_missing_required_vars(inline_env, &resolved);
        let mut invalid_choices = find_invalid_choices(inline_env, &resolved);

        // For dry-run, skip prompting and return issues without erroring
        if options.dry_run {
            return Ok(EnvResult {
                env: resolved,
                missing,
                invalid_choices,
            });
        }

        // Handle --prompt-all: prompt for ALL variables, showing current values
        if mode.should_prompt_all() {
            let mut sorted_keys: Vec<_> = inline_env.keys().collect();
            sorted_keys.sort();
            let editable_vars: Vec<EditableVar> = sorted_keys
                .iter()
                .map(|key| EditableVar {
                    name: (*key).clone(),
                    spec: inline_env[*key].clone(),
                    current_value: resolved.get(*key).cloned(),
                })
                .collect();

            let mut prompter = CliPrompter::new();
            let prompt_result = prompter.prompt_all(&editable_vars)?;

            // Apply all prompted values (replacing resolved)
            for (key, value) in &prompt_result.values {
                resolved.insert(key.clone(), value.clone());
            }

            // Save if requested
            if prompt_result.save {
                let path = saved_store.save(project_name, entity_name, &prompt_result.values)?;
                println!("Saved to {}", path.display());
            }

            // Re-check for issues after user has reviewed all
            missing = find_missing_required_vars(inline_env, &resolved);
            invalid_choices = find_invalid_choices(inline_env, &resolved);
        }
        // Otherwise, prompt only for missing vars
        else if !missing.is_empty() && mode.should_prompt() {
            let mut prompter = CliPrompter::new();
            let prompt_result = prompter.prompt_missing(&missing)?;

            // Apply prompted values
            for (key, value) in &prompt_result.values {
                resolved.insert(key.clone(), value.clone());
            }

            // Save if requested
            if prompt_result.save {
                let path = saved_store.save(project_name, entity_name, &prompt_result.values)?;
                println!("Saved to {}", path.display());
            }

            // Re-check for issues after prompting
            missing = find_missing_required_vars(inline_env, &resolved);
            invalid_choices = find_invalid_choices(inline_env, &resolved);
        }

        // Fail if still missing required vars
        if let Some(var) = missing.first() {
            return Err(MeriadocError::Validation(
                crate::core::validation::ValidationError::MissingRequiredEnv {
                    var: var.name.clone(),
                },
            ));
        }

        // Fail on invalid choice values
        if let Some(ic) = invalid_choices.first() {
            return Err(MeriadocError::Validation(
                crate::core::validation::ValidationError::InvalidChoice {
                    var: ic.var.clone(),
                    value: ic.value.clone(),
                    options: ic.options.clone(),
                },
            ));
        }

        Ok(EnvResult {
            env: resolved,
            missing: Vec::new(),
            invalid_choices: Vec::new(),
        })
    }

    pub fn run_task(
        app: &mut App,
        name: &str,
        options: &RunOptions,
        mode: InteractiveMode,
        exec_options: &ExecutionOptions,
    ) -> Result<TaskRunResult, MeriadocError> {
        // 1. Resolve the task
        let resolved = EntityResolver::resolve_task(name, &app.projects)?;
        let project_name = EntityResolver::project_name(resolved.project);

        // 2. Validate (with cache) - clone to avoid borrow conflict with `resolved`
        let spec_path = resolved.spec_file.path.clone();
        let spec_clone = resolved.spec_file.spec.clone();
        let project_root = resolved.project.root.clone();
        Self::validate_spec_file(&mut app.caches, &app.config.cache, &project_root, &spec_path, &spec_clone)?;

        // 3. Resolve environment (with interactive prompting if enabled, skipped for dry-run)
        let config_dir = app.config_parent_dir().to_path_buf();

        let env_result = Self::resolve_env_interactive(
            &resolved.spec.env,
            &resolved.spec.env_files,
            options,
            &resolved.project.root,
            project_name,
            name,
            mode,
            &config_dir,
        )?;

        // 4. Extract secrets and update execution options
        let secrets = extract_secrets(&resolved.spec.env);
        let exec_options = ExecutionOptions {
            secrets,
            ..exec_options.clone()
        };

        // 5. Resolve working directory
        let spec_file_dir = resolved
            .spec_file
            .path
            .parent()
            .ok_or_else(|| MeriadocError::Execution("invalid spec file path".to_string()))?;

        let workdir = CommandRunner::resolve_workdir(
            resolved.spec.workdir.as_deref(),
            &resolved.project.root,
            spec_file_dir,
            WorkdirMode::SpecFileDir,
        )?;

        // 6. Add special variables
        let mut env = env_result.env;
        Self::add_special_vars(&mut env, &resolved.project.root, spec_file_dir);

        // Dry-run: print what would happen and return
        if options.dry_run {
            Self::print_task_dry_run(
                name,
                project_name,
                &workdir.to_string_lossy(),
                &env,
                &env_result.missing,
                &env_result.invalid_choices,
                &resolved.spec.preconditions,
                &resolved.spec.cmds,
                resolved.spec.on_failure.as_ref().map(|p| &p.cmds),
            );
            return Ok(TaskRunResult {
                exit_code: 0,
                output: String::new(),
            });
        }

        // 7. Check preconditions
        for condition in &resolved.spec.preconditions {
            let results = CommandRunner::run_commands_with_options(
                &condition.cmds,
                &workdir,
                &env,
                &exec_options,
            )?;
            if results.iter().any(|r| !r.success) {
                // Handle on_failure if present
                if let Some(policy) = &condition.on_failure {
                    if !policy.cmds.is_empty() {
                        CommandRunner::run_commands_with_options(
                            &policy.cmds,
                            &workdir,
                            &env,
                            &exec_options,
                        )?;
                    }
                    if !policy.r#continue {
                        return Err(MeriadocError::PreconditionFailed {
                            entity: format!("task '{}'", name),
                            message: "precondition command failed".to_string(),
                        });
                    }
                } else {
                    return Err(MeriadocError::PreconditionFailed {
                        entity: format!("task '{}'", name),
                        message: "precondition command failed".to_string(),
                    });
                }
            }
        }

        // 8. Execute commands
        println!("Running task: {}", name);
        let results = CommandRunner::run_commands_with_options(
            &resolved.spec.cmds,
            &workdir,
            &env,
            &exec_options,
        )?;

        // 9. Collect output and exit code
        let mut output = String::new();
        let mut exit_code = 0;

        for result in &results {
            // Collect captured output (empty if capture_output was false)
            output.push_str(&result.stdout);
            output.push_str(&result.stderr);

            if !result.success {
                exit_code = result.exit_code;
            }
        }

        // Handle on_failure if task failed
        if exit_code != 0
            && let Some(policy) = &resolved.spec.on_failure
            && !policy.cmds.is_empty()
        {
            let failure_results = CommandRunner::run_commands_with_options(
                &policy.cmds,
                &workdir,
                &env,
                &exec_options,
            )?;
            // Also capture failure handler output
            for result in &failure_results {
                output.push_str(&result.stdout);
                output.push_str(&result.stderr);
            }
        }

        Ok(TaskRunResult { exit_code, output })
    }

    pub fn run_job(
        app: &mut App,
        name: &str,
        options: &RunOptions,
        mode: InteractiveMode,
        exec_options: &ExecutionOptions,
    ) -> Result<i32, MeriadocError> {
        // 1. Resolve the job
        let resolved = EntityResolver::resolve_job(name, &app.projects)?;
        let project_name = EntityResolver::project_name(resolved.project);

        // 2. Validate spec file - clone to avoid borrow conflict with `resolved`
        let spec_path = resolved.spec_file.path.clone();
        let spec_clone = resolved.spec_file.spec.clone();
        let project_root = resolved.project.root.clone();
        Self::validate_spec_file(&mut app.caches, &app.config.cache, &project_root, &spec_path, &spec_clone)?;

        // 3. Get spec file dir for special vars
        let spec_file_dir = resolved
            .spec_file
            .path
            .parent()
            .ok_or_else(|| MeriadocError::Execution("invalid spec file path".to_string()))?;

        // 4. Resolve job-level environment (base) with interactive prompting
        let config_dir = app.config_parent_dir().to_path_buf();

        let env_result = Self::resolve_env_interactive(
            &resolved.spec.env,
            &resolved.spec.env_files,
            options,
            &resolved.project.root,
            project_name,
            name,
            mode,
            &config_dir,
        )?;

        // Add special variables
        let mut job_env = env_result.env;
        Self::add_special_vars(&mut job_env, &resolved.project.root, spec_file_dir);

        // 5. Extract secrets from job env and all task envs
        let mut all_secrets = extract_secrets(&resolved.spec.env);
        for task_name in &resolved.spec.tasks {
            if let Some(task_spec) = resolved.spec_file.spec.tasks.get(task_name) {
                all_secrets.extend(extract_secrets(&task_spec.env));
            }
        }
        let exec_options = ExecutionOptions {
            secrets: all_secrets,
            ..exec_options.clone()
        };

        // Dry-run: print what would happen
        if options.dry_run {
            println!("[dry-run] Job: {}", name);
            println!("[dry-run] Project: {}", project_name);
            println!("[dry-run] Tasks: {}", resolved.spec.tasks.join(" → "));

            // Show missing/invalid env vars
            Self::print_env_issues(&env_result.missing, &env_result.invalid_choices);

            if !job_env.is_empty() {
                println!("[dry-run] Job environment:");
                let mut keys: Vec<_> = job_env.keys().collect();
                keys.sort();
                for key in keys {
                    println!("    {}={}", key, job_env.get(key).unwrap());
                }
            }

            if let Some(policy) = &resolved.spec.on_failure
                && !policy.cmds.is_empty()
            {
                println!("[dry-run] On failure:");
                for (i, cmd) in policy.cmds.iter().enumerate() {
                    let interpolated = CommandRunner::interpolate_command(cmd, &job_env);
                    Self::print_command_with_interpolation(i + 1, cmd, &interpolated);
                }
            }

            // Print each task
            for (idx, task_name) in resolved.spec.tasks.iter().enumerate() {
                let task_spec = resolved
                    .spec_file
                    .spec
                    .tasks
                    .get(task_name)
                    .ok_or_else(|| MeriadocError::EntityNotFound {
                        kind: "task".to_string(),
                        name: task_name.clone(),
                    })?;

                // Resolve task's env for interpolation display
                let task_base_env = EnvResolver::resolve(
                    &task_spec.env,
                    &task_spec.env_files,
                    &[],
                    &resolved.project.root,
                )?;
                let mut task_env = EnvResolver::merge_job_env(task_base_env, &resolved.spec.env);
                for (key, value) in &options.env {
                    task_env.insert(key.clone(), value.clone());
                }
                Self::add_special_vars(&mut task_env, &resolved.project.root, spec_file_dir);

                println!();
                println!(
                    "[dry-run] Task {}/{}: {}",
                    idx + 1,
                    resolved.spec.tasks.len(),
                    task_name
                );
                println!("[dry-run] Commands:");
                for (i, cmd) in task_spec.cmds.iter().enumerate() {
                    let interpolated = CommandRunner::interpolate_command(cmd, &task_env);
                    Self::print_command_with_interpolation(i + 1, cmd, &interpolated);
                }
            }

            return Ok(0);
        }

        println!(
            "Running job: {} ({} tasks)",
            name,
            resolved.spec.tasks.len()
        );

        // 6. Execute each task in order
        for task_name in &resolved.spec.tasks {
            // Find task in the same spec file (v0.1 constraint)
            let task_spec = resolved
                .spec_file
                .spec
                .tasks
                .get(task_name)
                .ok_or_else(|| MeriadocError::EntityNotFound {
                    kind: "task".to_string(),
                    name: task_name.clone(),
                })?;

            // Merge task env with job env (job takes precedence)
            let task_base_env = EnvResolver::resolve(
                &task_spec.env,
                &task_spec.env_files,
                &[], // CLI env already in job_env
                &resolved.project.root,
            )?;
            let merged_env = EnvResolver::merge_job_env(task_base_env, &resolved.spec.env);
            // Apply CLI env on top
            let mut final_env = merged_env;
            for (key, value) in &options.env {
                final_env.insert(key.clone(), value.clone());
            }
            // Add special variables
            Self::add_special_vars(&mut final_env, &resolved.project.root, spec_file_dir);

            let workdir = CommandRunner::resolve_workdir(
                task_spec.workdir.as_deref(),
                &resolved.project.root,
                spec_file_dir,
                WorkdirMode::SpecFileDir,
            )?;

            // Execute task commands
            println!("  Running task: {}", task_name);
            let results = CommandRunner::run_commands_with_options(
                &task_spec.cmds,
                &workdir,
                &final_env,
                &exec_options,
            )?;

            // Check for failure
            if let Some(result) = results.iter().find(|r| !r.success) {
                // Handle job-level on_failure
                if let Some(policy) = &resolved.spec.on_failure {
                    if !policy.cmds.is_empty() {
                        CommandRunner::run_commands_with_options(
                            &policy.cmds,
                            &workdir,
                            &final_env,
                            &exec_options,
                        )?;
                    }
                    if !policy.r#continue {
                        return Ok(result.exit_code);
                    }
                } else {
                    return Ok(result.exit_code);
                }
            }
        }

        println!("Job completed successfully: {}", name);
        Ok(0)
    }

    pub fn run_shell(
        app: &mut App,
        name: &str,
        options: &RunOptions,
        mode: InteractiveMode,
    ) -> Result<i32, MeriadocError> {
        // 1. Resolve the shell
        let resolved = EntityResolver::resolve_shell(name, &app.projects)?;
        let project_name = EntityResolver::project_name(resolved.project);

        // 2. Validate spec file - clone to avoid borrow conflict with `resolved`
        let spec_path = resolved.spec_file.path.clone();
        let spec_clone = resolved.spec_file.spec.clone();
        let project_root = resolved.project.root.clone();
        Self::validate_spec_file(&mut app.caches, &app.config.cache, &project_root, &spec_path, &spec_clone)?;

        // 3. Resolve environment with interactive prompting
        let config_dir = app.config_parent_dir().to_path_buf();

        let env_result = Self::resolve_env_interactive(
            &resolved.spec.env,
            &resolved.spec.env_files,
            options,
            &resolved.project.root,
            project_name,
            name,
            mode,
            &config_dir,
        )?;

        // 4. Resolve working directory (shells default to CWD)
        let spec_file_dir = resolved
            .spec_file
            .path
            .parent()
            .ok_or_else(|| MeriadocError::Execution("invalid spec file path".to_string()))?;

        let workdir = CommandRunner::resolve_workdir(
            resolved.spec.workdir.as_deref(),
            &resolved.project.root,
            spec_file_dir,
            WorkdirMode::CurrentDir, // Shells default to current dir
        )?;

        // 5. Add special variables
        let mut env = env_result.env;
        Self::add_special_vars(&mut env, &resolved.project.root, spec_file_dir);

        // Dry-run: print what would happen
        if options.dry_run {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
            println!("[dry-run] Shell: {}", name);
            println!("[dry-run] Project: {}", project_name);
            println!("[dry-run] Working directory: {}", workdir.display());
            println!("[dry-run] Shell program: {}", shell);

            // Show missing/invalid env vars
            Self::print_env_issues(&env_result.missing, &env_result.invalid_choices);

            if !env.is_empty() {
                println!("[dry-run] Environment:");
                let mut keys: Vec<_> = env.keys().collect();
                keys.sort();
                for key in keys {
                    println!("    {}={}", key, env.get(key).unwrap());
                }
            }

            if !resolved.spec.init_cmds.is_empty() {
                println!("[dry-run] Init commands:");
                for (i, cmd) in resolved.spec.init_cmds.iter().enumerate() {
                    let interpolated = CommandRunner::interpolate_command(cmd, &env);
                    Self::print_command_with_interpolation(i + 1, cmd, &interpolated);
                }
            }

            println!("[dry-run] Would start interactive {} session", shell);
            return Ok(0);
        }

        println!("Starting shell: {}", name);
        if !resolved.spec.init_cmds.is_empty() {
            println!("  Init commands: {}", resolved.spec.init_cmds.len());
        }

        // 6. Start interactive shell (this replaces the process)
        InteractiveShell::start(&workdir, &env, &resolved.spec.init_cmds, name, project_name)?;

        // If we get here, something went wrong (exec should not return)
        Ok(1)
    }

    /// Print a command, showing interpolated version if different.
    fn print_command_with_interpolation(index: usize, raw: &str, interpolated: &str) {
        print!(
            "{}",
            format_command_with_interpolation(index, raw, interpolated)
        );
    }

    /// Print environment issues for dry-run output.
    fn print_env_issues(missing: &[MissingVar], invalid_choices: &[InvalidChoice]) {
        print!("{}", format_env_issues(missing, invalid_choices));
    }

    #[allow(clippy::too_many_arguments)]
    fn print_task_dry_run(
        name: &str,
        project_name: &str,
        workdir: &str,
        env: &ResolvedEnv,
        missing: &[MissingVar],
        invalid_choices: &[InvalidChoice],
        preconditions: &[ConditionSpec],
        cmds: &[String],
        on_failure_cmds: Option<&Vec<String>>,
    ) {
        let output = format_task_dry_run(
            name,
            project_name,
            workdir,
            env,
            missing,
            invalid_choices,
            preconditions,
            cmds,
            on_failure_cmds,
        );
        print!("{}", output);
    }

    fn validate_spec_file(
        caches: &mut HashMap<PathBuf, ValidationCache>,
        cache_config: &CacheConfig,
        project_root: &Path,
        spec_path: &Path,
        spec: &crate::core::spec::SpecFile,
    ) -> Result<(), MeriadocError> {
        use crate::core::validation::ProjectValidator;

        if cache_config.enabled
            && let Some(cache) = caches.get(project_root)
            && !cache.needs_validation(spec_path)?
        {
            return Ok(()); // Already validated and unchanged
        }

        // Validate (clone needed because validate takes owned slice)
        #[allow(clippy::cloned_ref_to_slice_refs)]
        let result = ProjectValidator::validate(&[spec.clone()]);
        let is_valid = result.is_ok();

        // Update and persist the per-project cache
        if cache_config.enabled {
            let cache_dir = project_cache_dir(&cache_config.dir, project_root);
            let cache = caches.entry(project_root.to_path_buf()).or_default();
            cache.record_validation(spec_path, is_valid)?;
            cache.save(&cache_dir)?;
        }

        if !is_valid {
            for err in result.errors() {
                eprintln!("Validation error: {} - {}", err.context, err.error);
            }
            return Err(MeriadocError::Validation(
                crate::core::validation::ValidationError::UnsupportedVersion {
                    version: "invalid".to_string(),
                    file: spec_path.to_path_buf(),
                },
            ));
        }

        Ok(())
    }
}

/// Format command with interpolation for dry-run output.
fn format_command_with_interpolation(index: usize, raw: &str, interpolated: &str) -> String {
    if raw == interpolated {
        format!("    {}. {}\n", index, raw)
    } else {
        format!("    {}. {}\n       → {}\n", index, raw, interpolated)
    }
}

/// Format environment issues for dry-run output.
fn format_env_issues(missing: &[MissingVar], invalid_choices: &[InvalidChoice]) -> String {
    let mut output = String::new();

    if !missing.is_empty() {
        output.push_str("[dry-run] Missing required environment variables:\n");
        for var in missing {
            if !var.spec.options.is_empty() {
                output.push_str(&format!(
                    "    {} (options: {})\n",
                    var.name,
                    var.spec.options.join(", ")
                ));
            } else {
                output.push_str(&format!("    {} (type: {})\n", var.name, var.spec.var_type));
            }
        }
    }

    if !invalid_choices.is_empty() {
        output.push_str("[dry-run] Invalid environment variable values:\n");
        for ic in invalid_choices {
            output.push_str(&format!(
                "    {}={} (valid options: {})\n",
                ic.var,
                ic.value,
                ic.options.join(", ")
            ));
        }
    }

    output
}

/// Format task dry-run output as a string.
#[allow(clippy::too_many_arguments)]
fn format_task_dry_run(
    name: &str,
    project_name: &str,
    workdir: &str,
    env: &ResolvedEnv,
    missing: &[MissingVar],
    invalid_choices: &[InvalidChoice],
    preconditions: &[ConditionSpec],
    cmds: &[String],
    on_failure_cmds: Option<&Vec<String>>,
) -> String {
    use crate::core::execution::CommandRunner;

    let mut output = String::new();
    output.push_str(&format!("[dry-run] Task: {}\n", name));
    output.push_str(&format!("[dry-run] Project: {}\n", project_name));
    output.push_str(&format!("[dry-run] Working directory: {}\n", workdir));

    // Show missing/invalid env vars
    output.push_str(&format_env_issues(missing, invalid_choices));

    if !env.is_empty() {
        output.push_str("[dry-run] Environment:\n");
        let mut keys: Vec<_> = env.keys().collect();
        keys.sort();
        for key in keys {
            output.push_str(&format!("    {}={}\n", key, env.get(key).unwrap()));
        }
    }

    if !preconditions.is_empty() {
        output.push_str("[dry-run] Preconditions:\n");
        for (i, condition) in preconditions.iter().enumerate() {
            for cmd in &condition.cmds {
                let interpolated = CommandRunner::interpolate_command(cmd, env);
                output.push_str(&format_command_with_interpolation(
                    i + 1,
                    cmd,
                    &interpolated,
                ));
            }
        }
    }

    output.push_str("[dry-run] Commands:\n");
    for (i, cmd) in cmds.iter().enumerate() {
        let interpolated = CommandRunner::interpolate_command(cmd, env);
        output.push_str(&format_command_with_interpolation(
            i + 1,
            cmd,
            &interpolated,
        ));
    }

    if let Some(cmds) = on_failure_cmds
        && !cmds.is_empty()
    {
        output.push_str("[dry-run] On failure:\n");
        for (i, cmd) in cmds.iter().enumerate() {
            let interpolated = CommandRunner::interpolate_command(cmd, env);
            output.push_str(&format_command_with_interpolation(
                i + 1,
                cmd,
                &interpolated,
            ));
        }
    }

    output
}

/// Execute a task for MCP and return output as string.
///
/// This is a simplified interface for MCP tool calls that:
/// - Takes env overrides as key-value pairs
/// - Returns output/status as a string
/// - Never prompts interactively
pub fn run_task_for_mcp(
    app: &mut App,
    name: &str,
    cli_env: &[(String, String)],
    dry_run: bool,
) -> Result<String, MeriadocError> {
    // Build RunOptions from MCP parameters
    let options = RunOptions {
        env: cli_env.to_vec(),
        env_file: None,
        verbose: false,
        dry_run,
        no_interactive: true, // MCP never prompts
        interactive: false,
        prompt_all: false,
        timeout: None,
    };

    // Non-interactive mode for MCP
    let mode = InteractiveMode {
        no_interactive: true,
        interactive: false,
        prompt_all: false,
    };
    let exec_options = ExecutionOptions {
        verbose: false,
        timeout: None,
        capture_output: true, // MCP mode: capture output for agent
        ..Default::default()
    };

    // For dry-run, capture what would happen
    if dry_run {
        return run_task_dry_run_for_mcp(app, name, &options, mode);
    }

    // Execute the task and capture output
    let result = RunActions::run_task(app, name, &options, mode, &exec_options)?;

    // Build response with captured output
    let mut response = String::new();

    if !result.output.is_empty() {
        response.push_str(&result.output);
        if !result.output.ends_with('\n') {
            response.push('\n');
        }
    }

    if result.exit_code == 0 {
        response.push_str(&format!("[Task '{}' completed successfully]", name));
    } else {
        response.push_str(&format!(
            "[Task '{}' failed with exit code {}]",
            name, result.exit_code
        ));
    }

    Ok(response)
}

/// Generate dry-run output for MCP.
fn run_task_dry_run_for_mcp(
    app: &mut App,
    name: &str,
    options: &RunOptions,
    mode: InteractiveMode,
) -> Result<String, MeriadocError> {
    use crate::core::execution::CommandRunner;

    // Resolve the task
    let resolved = EntityResolver::resolve_task(name, &app.projects)?;
    let project_name = EntityResolver::project_name(resolved.project);

    // Get config directory
    let config_dir = app.config_parent_dir().to_path_buf();

    // Resolve environment
    let env_result = RunActions::resolve_env_interactive(
        &resolved.spec.env,
        &resolved.spec.env_files,
        options,
        &resolved.project.root,
        project_name,
        name,
        mode,
        &config_dir,
    )?;

    // Resolve working directory
    let spec_file_dir = resolved
        .spec_file
        .path
        .parent()
        .ok_or_else(|| MeriadocError::Execution("invalid spec file path".to_string()))?;

    let workdir = CommandRunner::resolve_workdir(
        resolved.spec.workdir.as_deref(),
        &resolved.project.root,
        spec_file_dir,
        WorkdirMode::SpecFileDir,
    )?;

    // Add special vars for interpolation display
    let mut env = env_result.env;
    RunActions::add_special_vars(&mut env, &resolved.project.root, spec_file_dir);

    // Use the shared formatter
    Ok(format_task_dry_run(
        name,
        project_name,
        &workdir.to_string_lossy(),
        &env,
        &env_result.missing,
        &env_result.invalid_choices,
        &resolved.spec.preconditions,
        &resolved.spec.cmds,
        resolved.spec.on_failure.as_ref().map(|p| &p.cmds),
    ))
}
