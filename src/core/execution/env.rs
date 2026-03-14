use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::core::spec::EnvVarSpec;
use crate::core::validation::MeriadocError;

/// Resolved environment variables ready for execution
pub type ResolvedEnv = HashMap<String, String>;

/// Information about a missing required environment variable
#[derive(Debug, Clone)]
pub struct MissingVar {
    /// Variable name
    pub name: String,
    /// The spec defining this variable
    pub spec: EnvVarSpec,
}

/// Information about a choice validation error
#[derive(Debug, Clone)]
pub struct InvalidChoiceVar {
    /// Variable name
    pub name: String,
    /// The invalid value provided
    pub value: String,
    /// Valid options
    pub options: Vec<String>,
}

/// Result of environment resolution (before validation)
#[derive(Debug)]
pub struct EnvResolution {
    /// Successfully resolved variables
    pub resolved: ResolvedEnv,
    /// Required variables that are missing
    pub missing: Vec<MissingVar>,
    /// Variables with invalid choice values
    pub invalid_choices: Vec<InvalidChoiceVar>,
}

impl EnvResolution {
    /// Convert to Result - fails if there are missing vars or invalid choices
    pub fn into_result(self) -> Result<ResolvedEnv, MeriadocError> {
        // Check for invalid choices first (more specific error)
        if let Some(invalid) = self.invalid_choices.first() {
            return Err(MeriadocError::Validation(
                crate::core::validation::ValidationError::InvalidChoice {
                    var: invalid.name.clone(),
                    value: invalid.value.clone(),
                    options: invalid.options.clone(),
                },
            ));
        }

        // Check for missing required vars
        if let Some(missing) = self.missing.first() {
            return Err(MeriadocError::Validation(
                crate::core::validation::ValidationError::MissingRequiredEnv {
                    var: missing.name.clone(),
                },
            ));
        }

        Ok(self.resolved)
    }
}

pub struct EnvResolver;

impl EnvResolver {
    /// Resolve environment variables without failing on missing required vars.
    /// Returns an EnvResolution that can be inspected for missing vars before
    /// being converted to a Result.
    ///
    /// Priority:
    /// 1. CLI --env (highest)
    /// 2. Inline env from spec
    /// 3. env_files (lowest, loaded in order)
    pub fn resolve_partial(
        inline_env: &HashMap<String, EnvVarSpec>,
        env_files: &[String],
        cli_env: &[(String, String)],
        project_root: &Path,
    ) -> Result<EnvResolution, MeriadocError> {
        let mut resolved = ResolvedEnv::new();

        // 1. Load env_files (lowest priority)
        for env_file in env_files {
            let path = project_root.join(env_file);
            if path.exists() {
                let file_env = Self::load_env_file(&path)?;
                for (key, value) in file_env {
                    resolved.entry(key).or_insert(value);
                }
            }
        }

        // 2. Apply inline env (override env_files)
        for (key, spec) in inline_env {
            if let Some(default) = &spec.default {
                resolved.insert(key.clone(), default.clone());
            }
        }

        // 3. Apply CLI env (highest priority)
        for (key, value) in cli_env {
            resolved.insert(key.clone(), value.clone());
        }

        // 4. Collect missing required variables (don't fail yet)
        let mut missing = Vec::new();
        for (key, spec) in inline_env {
            if spec.required && !resolved.contains_key(key) {
                missing.push(MissingVar {
                    name: key.clone(),
                    spec: spec.clone(),
                });
            }
        }

        // 5. Collect invalid choice values (don't fail yet)
        let mut invalid_choices = Vec::new();
        for (key, spec) in inline_env {
            if !spec.options.is_empty()
                && let Some(value) = resolved.get(key)
                && !spec.options.contains(value)
            {
                invalid_choices.push(InvalidChoiceVar {
                    name: key.clone(),
                    value: value.clone(),
                    options: spec.options.clone(),
                });
            }
        }

        Ok(EnvResolution {
            resolved,
            missing,
            invalid_choices,
        })
    }

    /// Merge environment variables with priority and validate.
    /// This is a convenience method that calls resolve_partial and validates.
    ///
    /// Priority:
    /// 1. CLI --env (highest)
    /// 2. Inline env from spec
    /// 3. env_files (lowest, loaded in order)
    pub fn resolve(
        inline_env: &HashMap<String, EnvVarSpec>,
        env_files: &[String],
        cli_env: &[(String, String)],
        project_root: &Path,
    ) -> Result<ResolvedEnv, MeriadocError> {
        let resolution = Self::resolve_partial(inline_env, env_files, cli_env, project_root)?;
        resolution.into_result()
    }

    /// Load a .env file into a HashMap
    pub fn load_env_file(path: &Path) -> Result<HashMap<String, String>, MeriadocError> {
        let contents = fs::read_to_string(path)?;
        let mut env = HashMap::new();

        for line in contents.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse KEY=VALUE
            if let Some(pos) = line.find('=') {
                let key = line[..pos].trim().to_string();
                let value = line[pos + 1..].trim();

                // Handle quoted values
                let value = if (value.starts_with('"') && value.ends_with('"'))
                    || (value.starts_with('\'') && value.ends_with('\''))
                {
                    value[1..value.len() - 1].to_string()
                } else {
                    value.to_string()
                };

                env.insert(key, value);
            }
        }

        Ok(env)
    }

    /// Merge job-level env over task-level env
    pub fn merge_job_env(
        mut task_env: ResolvedEnv,
        job_env: &HashMap<String, EnvVarSpec>,
    ) -> ResolvedEnv {
        for (key, spec) in job_env {
            if let Some(default) = &spec.default {
                task_env.insert(key.clone(), default.clone());
            }
        }
        task_env
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::spec::VarType;
    use std::io::Write;
    use tempfile::TempDir;

    fn make_env_spec(default: Option<&str>, required: bool) -> EnvVarSpec {
        EnvVarSpec {
            var_type: VarType::String,
            default: default.map(|s| s.to_string()),
            options: vec![],
            required,
        }
    }

    fn make_choice_spec(default: Option<&str>, options: &[&str]) -> EnvVarSpec {
        EnvVarSpec {
            var_type: VarType::Choice,
            default: default.map(|s| s.to_string()),
            options: options.iter().map(|s| s.to_string()).collect(),
            required: false,
        }
    }

    #[test]
    fn test_resolve_empty_inputs() {
        let result = EnvResolver::resolve(&HashMap::new(), &[], &[], Path::new("/tmp"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_resolve_inline_env_with_defaults() {
        let mut inline = HashMap::new();
        inline.insert("FOO".to_string(), make_env_spec(Some("bar"), false));
        inline.insert("BAZ".to_string(), make_env_spec(Some("qux"), false));

        let result = EnvResolver::resolve(&inline, &[], &[], Path::new("/tmp")).unwrap();

        assert_eq!(result.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(result.get("BAZ"), Some(&"qux".to_string()));
    }

    #[test]
    fn test_resolve_cli_overrides_inline() {
        let mut inline = HashMap::new();
        inline.insert("FOO".to_string(), make_env_spec(Some("from_inline"), false));

        let cli_env = vec![("FOO".to_string(), "from_cli".to_string())];

        let result = EnvResolver::resolve(&inline, &[], &cli_env, Path::new("/tmp")).unwrap();

        assert_eq!(result.get("FOO"), Some(&"from_cli".to_string()));
    }

    #[test]
    fn test_resolve_inline_overrides_env_file() {
        let temp_dir = TempDir::new().unwrap();
        let env_file_path = temp_dir.path().join(".env");
        let mut file = std::fs::File::create(&env_file_path).unwrap();
        writeln!(file, "FOO=from_file").unwrap();

        let mut inline = HashMap::new();
        inline.insert("FOO".to_string(), make_env_spec(Some("from_inline"), false));

        let result =
            EnvResolver::resolve(&inline, &[".env".to_string()], &[], temp_dir.path()).unwrap();

        assert_eq!(result.get("FOO"), Some(&"from_inline".to_string()));
    }

    #[test]
    fn test_resolve_cli_overrides_env_file() {
        let temp_dir = TempDir::new().unwrap();
        let env_file_path = temp_dir.path().join(".env");
        let mut file = std::fs::File::create(&env_file_path).unwrap();
        writeln!(file, "FOO=from_file").unwrap();

        let cli_env = vec![("FOO".to_string(), "from_cli".to_string())];

        let result = EnvResolver::resolve(
            &HashMap::new(),
            &[".env".to_string()],
            &cli_env,
            temp_dir.path(),
        )
        .unwrap();

        assert_eq!(result.get("FOO"), Some(&"from_cli".to_string()));
    }

    #[test]
    fn test_resolve_full_priority_chain() {
        // Test: CLI > inline > env_file
        let temp_dir = TempDir::new().unwrap();
        let env_file_path = temp_dir.path().join(".env");
        let mut file = std::fs::File::create(&env_file_path).unwrap();
        writeln!(file, "A=file_a").unwrap();
        writeln!(file, "B=file_b").unwrap();
        writeln!(file, "C=file_c").unwrap();

        let mut inline = HashMap::new();
        inline.insert("B".to_string(), make_env_spec(Some("inline_b"), false));
        inline.insert("C".to_string(), make_env_spec(Some("inline_c"), false));

        let cli_env = vec![("C".to_string(), "cli_c".to_string())];

        let result =
            EnvResolver::resolve(&inline, &[".env".to_string()], &cli_env, temp_dir.path())
                .unwrap();

        assert_eq!(result.get("A"), Some(&"file_a".to_string())); // from file only
        assert_eq!(result.get("B"), Some(&"inline_b".to_string())); // inline overrides file
        assert_eq!(result.get("C"), Some(&"cli_c".to_string())); // CLI overrides all
    }

    #[test]
    fn test_resolve_required_var_missing_errors() {
        let mut inline = HashMap::new();
        inline.insert("REQUIRED_VAR".to_string(), make_env_spec(None, true));

        let result = EnvResolver::resolve(&inline, &[], &[], Path::new("/tmp"));

        assert!(result.is_err());
        match result.unwrap_err() {
            MeriadocError::Validation(
                crate::core::validation::ValidationError::MissingRequiredEnv { var },
            ) => {
                assert_eq!(var, "REQUIRED_VAR");
            }
            _ => panic!("Expected MissingRequiredEnv error"),
        }
    }

    #[test]
    fn test_resolve_required_var_with_default_ok() {
        let mut inline = HashMap::new();
        inline.insert(
            "REQUIRED_VAR".to_string(),
            make_env_spec(Some("default_value"), true),
        );

        let result = EnvResolver::resolve(&inline, &[], &[], Path::new("/tmp")).unwrap();

        assert_eq!(
            result.get("REQUIRED_VAR"),
            Some(&"default_value".to_string())
        );
    }

    #[test]
    fn test_resolve_required_var_from_cli_ok() {
        let mut inline = HashMap::new();
        inline.insert("REQUIRED_VAR".to_string(), make_env_spec(None, true));

        let cli_env = vec![("REQUIRED_VAR".to_string(), "provided".to_string())];

        let result = EnvResolver::resolve(&inline, &[], &cli_env, Path::new("/tmp")).unwrap();

        assert_eq!(result.get("REQUIRED_VAR"), Some(&"provided".to_string()));
    }

    #[test]
    fn test_env_file_parsing_comments() {
        let temp_dir = TempDir::new().unwrap();
        let env_file_path = temp_dir.path().join(".env");
        let mut file = std::fs::File::create(&env_file_path).unwrap();
        writeln!(file, "# This is a comment").unwrap();
        writeln!(file, "FOO=bar").unwrap();
        writeln!(file, "# Another comment").unwrap();
        writeln!(file, "BAZ=qux").unwrap();

        let result =
            EnvResolver::resolve(&HashMap::new(), &[".env".to_string()], &[], temp_dir.path())
                .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(result.get("BAZ"), Some(&"qux".to_string()));
    }

    #[test]
    fn test_env_file_parsing_empty_lines() {
        let temp_dir = TempDir::new().unwrap();
        let env_file_path = temp_dir.path().join(".env");
        let mut file = std::fs::File::create(&env_file_path).unwrap();
        writeln!(file, "FOO=bar").unwrap();
        writeln!(file).unwrap();
        writeln!(file, "   ").unwrap();
        writeln!(file, "BAZ=qux").unwrap();

        let result =
            EnvResolver::resolve(&HashMap::new(), &[".env".to_string()], &[], temp_dir.path())
                .unwrap();

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_env_file_parsing_double_quotes() {
        let temp_dir = TempDir::new().unwrap();
        let env_file_path = temp_dir.path().join(".env");
        let mut file = std::fs::File::create(&env_file_path).unwrap();
        writeln!(file, r#"FOO="hello world""#).unwrap();

        let result =
            EnvResolver::resolve(&HashMap::new(), &[".env".to_string()], &[], temp_dir.path())
                .unwrap();

        assert_eq!(result.get("FOO"), Some(&"hello world".to_string()));
    }

    #[test]
    fn test_env_file_parsing_single_quotes() {
        let temp_dir = TempDir::new().unwrap();
        let env_file_path = temp_dir.path().join(".env");
        let mut file = std::fs::File::create(&env_file_path).unwrap();
        writeln!(file, "FOO='hello world'").unwrap();

        let result =
            EnvResolver::resolve(&HashMap::new(), &[".env".to_string()], &[], temp_dir.path())
                .unwrap();

        assert_eq!(result.get("FOO"), Some(&"hello world".to_string()));
    }

    #[test]
    fn test_env_file_parsing_value_with_equals() {
        let temp_dir = TempDir::new().unwrap();
        let env_file_path = temp_dir.path().join(".env");
        let mut file = std::fs::File::create(&env_file_path).unwrap();
        writeln!(file, "CONNECTION=host=localhost;port=5432").unwrap();

        let result =
            EnvResolver::resolve(&HashMap::new(), &[".env".to_string()], &[], temp_dir.path())
                .unwrap();

        assert_eq!(
            result.get("CONNECTION"),
            Some(&"host=localhost;port=5432".to_string())
        );
    }

    #[test]
    fn test_env_file_missing_is_ignored() {
        let result = EnvResolver::resolve(
            &HashMap::new(),
            &["nonexistent.env".to_string()],
            &[],
            Path::new("/tmp"),
        );

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_multiple_env_files_first_wins() {
        let temp_dir = TempDir::new().unwrap();

        let env1_path = temp_dir.path().join("first.env");
        let mut file1 = std::fs::File::create(&env1_path).unwrap();
        writeln!(file1, "FOO=first").unwrap();

        let env2_path = temp_dir.path().join("second.env");
        let mut file2 = std::fs::File::create(&env2_path).unwrap();
        writeln!(file2, "FOO=second").unwrap();
        writeln!(file2, "BAR=only_in_second").unwrap();

        let result = EnvResolver::resolve(
            &HashMap::new(),
            &["first.env".to_string(), "second.env".to_string()],
            &[],
            temp_dir.path(),
        )
        .unwrap();

        // First file wins for FOO (or_insert behavior)
        assert_eq!(result.get("FOO"), Some(&"first".to_string()));
        // BAR only in second, so it's added
        assert_eq!(result.get("BAR"), Some(&"only_in_second".to_string()));
    }

    #[test]
    fn test_merge_job_env_overrides_task() {
        let mut task_env = ResolvedEnv::new();
        task_env.insert("A".to_string(), "task_a".to_string());
        task_env.insert("B".to_string(), "task_b".to_string());

        let mut job_env = HashMap::new();
        job_env.insert("B".to_string(), make_env_spec(Some("job_b"), false));
        job_env.insert("C".to_string(), make_env_spec(Some("job_c"), false));

        let result = EnvResolver::merge_job_env(task_env, &job_env);

        assert_eq!(result.get("A"), Some(&"task_a".to_string())); // unchanged
        assert_eq!(result.get("B"), Some(&"job_b".to_string())); // overridden
        assert_eq!(result.get("C"), Some(&"job_c".to_string())); // added
    }

    #[test]
    fn test_merge_job_env_no_default_skipped() {
        let mut task_env = ResolvedEnv::new();
        task_env.insert("A".to_string(), "task_a".to_string());

        let mut job_env = HashMap::new();
        // Job env var with no default - should not override
        job_env.insert("A".to_string(), make_env_spec(None, false));

        let result = EnvResolver::merge_job_env(task_env, &job_env);

        // A remains from task since job has no default
        assert_eq!(result.get("A"), Some(&"task_a".to_string()));
    }

    #[test]
    fn test_choice_validation_valid_default() {
        let mut inline = HashMap::new();
        inline.insert(
            "ENV".to_string(),
            make_choice_spec(Some("dev"), &["dev", "staging", "prod"]),
        );

        let result = EnvResolver::resolve(&inline, &[], &[], Path::new("/tmp"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get("ENV"), Some(&"dev".to_string()));
    }

    #[test]
    fn test_choice_validation_valid_cli_override() {
        let mut inline = HashMap::new();
        inline.insert(
            "ENV".to_string(),
            make_choice_spec(Some("dev"), &["dev", "staging", "prod"]),
        );

        let cli_env = vec![("ENV".to_string(), "prod".to_string())];

        let result = EnvResolver::resolve(&inline, &[], &cli_env, Path::new("/tmp"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get("ENV"), Some(&"prod".to_string()));
    }

    #[test]
    fn test_choice_validation_invalid_value_errors() {
        let mut inline = HashMap::new();
        inline.insert(
            "ENV".to_string(),
            make_choice_spec(Some("dev"), &["dev", "staging", "prod"]),
        );

        let cli_env = vec![("ENV".to_string(), "invalid".to_string())];

        let result = EnvResolver::resolve(&inline, &[], &cli_env, Path::new("/tmp"));
        assert!(result.is_err());
        match result.unwrap_err() {
            MeriadocError::Validation(
                crate::core::validation::ValidationError::InvalidChoice {
                    var,
                    value,
                    options,
                },
            ) => {
                assert_eq!(var, "ENV");
                assert_eq!(value, "invalid");
                assert_eq!(options, vec!["dev", "staging", "prod"]);
            }
            _ => panic!("Expected InvalidChoice error"),
        }
    }

    #[test]
    fn test_choice_validation_no_options_skips_validation() {
        // If options is empty, no validation should happen
        let mut inline = HashMap::new();
        inline.insert("FOO".to_string(), make_env_spec(Some("anything"), false));

        let cli_env = vec![("FOO".to_string(), "literally_anything".to_string())];

        let result = EnvResolver::resolve(&inline, &[], &cli_env, Path::new("/tmp"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_choice_validation_undefined_var_skips() {
        // If the var is not set (no default, no CLI), choice validation should not error
        let mut inline = HashMap::new();
        inline.insert(
            "ENV".to_string(),
            EnvVarSpec {
                var_type: VarType::Choice,
                default: None,
                options: vec!["dev".to_string(), "prod".to_string()],
                required: false,
            },
        );

        let result = EnvResolver::resolve(&inline, &[], &[], Path::new("/tmp"));
        assert!(result.is_ok());
        assert!(result.unwrap().get("ENV").is_none());
    }
}
