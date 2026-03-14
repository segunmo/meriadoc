use std::fs;

use crate::core::spec::SpecFile;
use crate::core::validation::MeriadocError;
use crate::repo::project::{LoadedSpec, Project};

pub struct ProjectLoader;

impl ProjectLoader {
    pub fn load(mut project: Project) -> Result<Project, MeriadocError> {
        let mut specs = Vec::with_capacity(project.spec_files.len());

        for path in &project.spec_files {
            let contents = fs::read_to_string(path)?;
            let spec: SpecFile = serde_yaml::from_str(&contents)?;
            specs.push(LoadedSpec {
                path: path.clone(),
                spec,
            });
        }

        project.specs = specs;
        Ok(project)
    }
}
