use std::fs;
use std::path::Path;

use crate::repo::project::Project;

pub struct ProjectDiscovery;

impl ProjectDiscovery {
    pub fn discover(root: &Path, spec_files: &[impl AsRef<str>], max_depth: usize) -> Vec<Project> {
        let mut projects = Vec::new();
        Self::walk(root, root, spec_files, max_depth, &mut projects);
        projects
    }

    fn walk(
        current: &Path,
        _root: &Path, // Preserved for future use (e.g., relative path calculation)
        spec_files: &[impl AsRef<str>],
        remaining_depth: usize,
        projects: &mut Vec<Project>,
    ) {
        if remaining_depth == 0 {
            return;
        }

        let mut found_specs = Vec::new();

        if let Ok(entries) = fs::read_dir(current) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.is_file() && Self::is_spec_file(&path, spec_files) {
                    found_specs.push(path);
                }
            }
        }

        if !found_specs.is_empty() {
            projects.push(Project {
                root: current.to_path_buf(),
                spec_files: found_specs,
                specs: Vec::new(),
            });
            return; // stop recursion at project root
        }

        if let Ok(entries) = fs::read_dir(current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::walk(&path, _root, spec_files, remaining_depth - 1, projects);
                }
            }
        }
    }

    fn is_spec_file(path: &Path, spec_files: &[impl AsRef<str>]) -> bool {
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => return false,
        };

        spec_files.iter().any(|s| s.as_ref() == name)
    }
}
