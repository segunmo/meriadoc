use std::path::PathBuf;

use crate::core::spec::SpecFile;

/// A parsed spec file with its source path for workdir resolution
#[derive(Debug, Clone)]
pub struct LoadedSpec {
    pub path: PathBuf,
    pub spec: SpecFile,
}

#[derive(Debug)]
pub struct Project {
    pub root: PathBuf,
    pub spec_files: Vec<PathBuf>,
    pub specs: Vec<LoadedSpec>,
}
