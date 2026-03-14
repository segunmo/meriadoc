use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::validation::MeriadocError;

/// Manages saved environment files in the meriadoc config directory.
///
/// Structure: ~/.config/meriadGo ahoc/env/<project>/<entity>.env
pub struct SavedEnvStore {
    base_dir: PathBuf,
}

impl SavedEnvStore {
    /// Create a new SavedEnvStore with the given base directory
    pub fn new(config_dir: &Path) -> Self {
        Self {
            base_dir: config_dir.join("env"),
        }
    }

    /// Get the path to a saved env file for a project/entity
    fn env_path(&self, project: &str, entity: &str) -> PathBuf {
        self.base_dir.join(project).join(format!("{}.env", entity))
    }

    /// Load saved environment variables for a project/entity
    pub fn load(
        &self,
        project: &str,
        entity: &str,
    ) -> Result<HashMap<String, String>, MeriadocError> {
        let path = self.env_path(project, entity);
        if !path.exists() {
            return Ok(HashMap::new());
        }

        let contents = fs::read_to_string(&path)?;
        Ok(Self::parse_env_file(&contents))
    }

    /// Save environment variables for a project/entity
    pub fn save(
        &self,
        project: &str,
        entity: &str,
        env: &HashMap<String, String>,
    ) -> Result<PathBuf, MeriadocError> {
        let path = self.env_path(project, entity);

        // Create parent directories
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Format and write
        let contents = Self::format_env_file(env);
        fs::write(&path, contents)?;

        Ok(path)
    }

    /// Check if a saved env file exists
    pub fn exists(&self, project: &str, entity: &str) -> bool {
        self.env_path(project, entity).exists()
    }

    /// Delete a saved env file
    pub fn delete(&self, project: &str, entity: &str) -> Result<(), MeriadocError> {
        let path = self.env_path(project, entity);
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// List all saved env files for a project
    pub fn list_for_project(&self, project: &str) -> Result<Vec<String>, MeriadocError> {
        let project_dir = self.base_dir.join(project);
        if !project_dir.exists() {
            return Ok(Vec::new());
        }

        let mut entities = Vec::new();
        for entry in fs::read_dir(&project_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "env")
                && let Some(stem) = path.file_stem()
            {
                entities.push(stem.to_string_lossy().to_string());
            }
        }
        Ok(entities)
    }

    /// List all projects with saved env files
    pub fn list_projects(&self) -> Result<Vec<String>, MeriadocError> {
        if !self.base_dir.exists() {
            return Ok(Vec::new());
        }

        let mut projects = Vec::new();
        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                projects.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        Ok(projects)
    }

    /// Parse a .env file content into a HashMap
    fn parse_env_file(contents: &str) -> HashMap<String, String> {
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

        env
    }

    /// Format a HashMap as .env file content
    fn format_env_file(env: &HashMap<String, String>) -> String {
        let mut lines: Vec<String> = env
            .iter()
            .map(|(key, value)| {
                // Quote values that contain spaces or special characters
                if value.contains(' ') || value.contains('=') || value.contains('#') {
                    format!("{}=\"{}\"", key, value.replace('"', "\\\""))
                } else {
                    format!("{}={}", key, value)
                }
            })
            .collect();

        // Sort for consistent output
        lines.sort();
        lines.join("\n") + "\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let store = SavedEnvStore::new(temp_dir.path());

        let mut env = HashMap::new();
        env.insert("FOO".to_string(), "bar".to_string());
        env.insert("BAZ".to_string(), "qux".to_string());

        store.save("myproject", "mytask", &env).unwrap();

        let loaded = store.load("myproject", "mytask").unwrap();
        assert_eq!(loaded.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(loaded.get("BAZ"), Some(&"qux".to_string()));
    }

    #[test]
    fn test_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let store = SavedEnvStore::new(temp_dir.path());

        let loaded = store.load("noproject", "notask").unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_exists() {
        let temp_dir = TempDir::new().unwrap();
        let store = SavedEnvStore::new(temp_dir.path());

        assert!(!store.exists("myproject", "mytask"));

        let mut env = HashMap::new();
        env.insert("FOO".to_string(), "bar".to_string());
        store.save("myproject", "mytask", &env).unwrap();

        assert!(store.exists("myproject", "mytask"));
    }

    #[test]
    fn test_delete() {
        let temp_dir = TempDir::new().unwrap();
        let store = SavedEnvStore::new(temp_dir.path());

        let mut env = HashMap::new();
        env.insert("FOO".to_string(), "bar".to_string());
        store.save("myproject", "mytask", &env).unwrap();

        assert!(store.exists("myproject", "mytask"));
        store.delete("myproject", "mytask").unwrap();
        assert!(!store.exists("myproject", "mytask"));
    }

    #[test]
    fn test_list_for_project() {
        let temp_dir = TempDir::new().unwrap();
        let store = SavedEnvStore::new(temp_dir.path());

        let env = HashMap::new();
        store.save("myproject", "task1", &env).unwrap();
        store.save("myproject", "task2", &env).unwrap();
        store.save("otherproject", "task3", &env).unwrap();

        let mut entities = store.list_for_project("myproject").unwrap();
        entities.sort();
        assert_eq!(entities, vec!["task1", "task2"]);
    }

    #[test]
    fn test_list_projects() {
        let temp_dir = TempDir::new().unwrap();
        let store = SavedEnvStore::new(temp_dir.path());

        let env = HashMap::new();
        store.save("project1", "task", &env).unwrap();
        store.save("project2", "task", &env).unwrap();

        let mut projects = store.list_projects().unwrap();
        projects.sort();
        assert_eq!(projects, vec!["project1", "project2"]);
    }

    #[test]
    fn test_quoted_values() {
        let temp_dir = TempDir::new().unwrap();
        let store = SavedEnvStore::new(temp_dir.path());

        let mut env = HashMap::new();
        env.insert("SPACED".to_string(), "hello world".to_string());
        env.insert("WITH_EQUALS".to_string(), "key=value".to_string());

        store.save("myproject", "mytask", &env).unwrap();

        let loaded = store.load("myproject", "mytask").unwrap();
        assert_eq!(loaded.get("SPACED"), Some(&"hello world".to_string()));
        assert_eq!(loaded.get("WITH_EQUALS"), Some(&"key=value".to_string()));
    }
}
