use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::core::validation::MeriadocError;

/// Returns the per-project cache directory for a given project root.
///
/// Layout: `<base_cache_dir>/<project-name>-<hash8>/`
///
/// The path is canonicalized before hashing so that symlinks and relative
/// components don't produce different slugs for the same physical directory.
/// Falls back to the original path if canonicalization fails (e.g. in tests).
pub fn project_cache_dir(base_cache_dir: &Path, project_root: &Path) -> PathBuf {
    let canonical = fs::canonicalize(project_root).unwrap_or_else(|_| project_root.to_path_buf());

    let name = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    let slug = format!("{}-{}", name, &hash[..8]);

    base_cache_dir.join(slug)
}

/// Cache entry for a validated spec file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub file_hash: String,
    pub validated_at: u64,
    pub is_valid: bool,
}

/// Validation cache stored on disk
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValidationCache {
    entries: HashMap<PathBuf, CacheEntry>,
}

impl ValidationCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Load cache from disk, or create empty if not exists
    pub fn load(cache_dir: &Path) -> Result<Self, MeriadocError> {
        let cache_file = cache_dir.join("validation_cache.json");

        if !cache_file.exists() {
            return Ok(Self::new());
        }

        let contents = fs::read_to_string(&cache_file)?;
        let cache: ValidationCache = serde_json::from_str(&contents).map_err(|e| {
            MeriadocError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })?;

        Ok(cache)
    }

    /// Save cache to disk
    pub fn save(&self, cache_dir: &Path) -> Result<(), MeriadocError> {
        fs::create_dir_all(cache_dir)?;
        let cache_file = cache_dir.join("validation_cache.json");
        let serialized = serde_json::to_string_pretty(self).map_err(|e| {
            MeriadocError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            ))
        })?;
        fs::write(&cache_file, serialized)?;
        Ok(())
    }

    /// Compute SHA-256 hash of file contents
    pub fn hash_file(path: &Path) -> Result<String, MeriadocError> {
        let contents = fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&contents);
        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    /// Check if a file needs validation (returns true if not cached or changed)
    pub fn needs_validation(&self, path: &Path) -> Result<bool, MeriadocError> {
        let current_hash = Self::hash_file(path)?;

        match self.entries.get(path) {
            Some(entry) if entry.file_hash == current_hash && entry.is_valid => {
                Ok(false) // Cached and unchanged
            }
            _ => Ok(true), // Not cached or changed
        }
    }

    /// Record validation result for a file
    pub fn record_validation(&mut self, path: &Path, is_valid: bool) -> Result<(), MeriadocError> {
        let file_hash = Self::hash_file(path)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.entries.insert(
            path.to_path_buf(),
            CacheEntry {
                file_hash,
                validated_at: now,
                is_valid,
            },
        );

        Ok(())
    }

    /// Get cached validation status for a file
    #[cfg(test)]
    pub fn get(&self, path: &Path) -> Option<&CacheEntry> {
        self.entries.get(path)
    }

    /// Clear all cache entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// List all cached files
    pub fn list(&self) -> impl Iterator<Item = (&PathBuf, &CacheEntry)> {
        self.entries.iter()
    }

    /// Number of cached entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_new_cache_is_empty() {
        let cache = ValidationCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_hash_file_deterministic() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "hello world").unwrap();

        let hash1 = ValidationCache::hash_file(&file_path).unwrap();
        let hash2 = ValidationCache::hash_file(&file_path).unwrap();

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex is 64 chars
    }

    #[test]
    fn test_hash_file_changes_with_content() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        fs::write(&file_path, "content1").unwrap();
        let hash1 = ValidationCache::hash_file(&file_path).unwrap();

        fs::write(&file_path, "content2").unwrap();
        let hash2 = ValidationCache::hash_file(&file_path).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_file_nonexistent_errors() {
        let result = ValidationCache::hash_file(Path::new("/nonexistent/file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_needs_validation_uncached() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.yaml");
        fs::write(&file_path, "version: v1").unwrap();

        let cache = ValidationCache::new();
        let needs = cache.needs_validation(&file_path).unwrap();

        assert!(needs); // Not cached, needs validation
    }

    #[test]
    fn test_needs_validation_cached_valid() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.yaml");
        fs::write(&file_path, "version: v1").unwrap();

        let mut cache = ValidationCache::new();
        cache.record_validation(&file_path, true).unwrap();

        let needs = cache.needs_validation(&file_path).unwrap();
        assert!(!needs); // Cached and valid, no need to validate
    }

    #[test]
    fn test_needs_validation_cached_invalid() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.yaml");
        fs::write(&file_path, "version: v1").unwrap();

        let mut cache = ValidationCache::new();
        cache.record_validation(&file_path, false).unwrap(); // Recorded as invalid

        let needs = cache.needs_validation(&file_path).unwrap();
        assert!(needs); // Cached but invalid, needs re-validation
    }

    #[test]
    fn test_needs_validation_file_changed() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.yaml");
        fs::write(&file_path, "version: v1").unwrap();

        let mut cache = ValidationCache::new();
        cache.record_validation(&file_path, true).unwrap();

        // Modify file
        fs::write(&file_path, "version: v2").unwrap();

        let needs = cache.needs_validation(&file_path).unwrap();
        assert!(needs); // File changed, needs re-validation
    }

    #[test]
    fn test_record_and_get() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.yaml");
        fs::write(&file_path, "version: v1").unwrap();

        let mut cache = ValidationCache::new();
        cache.record_validation(&file_path, true).unwrap();

        let entry = cache.get(&file_path);
        assert!(entry.is_some());

        let entry = entry.unwrap();
        assert!(entry.is_valid);
        assert!(!entry.file_hash.is_empty());
        assert!(entry.validated_at > 0);
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("cache");
        let file_path = temp_dir.path().join("test.yaml");
        fs::write(&file_path, "version: v1").unwrap();

        // Create cache with entry
        let mut cache = ValidationCache::new();
        cache.record_validation(&file_path, true).unwrap();
        cache.save(&cache_dir).unwrap();

        // Load from disk
        let loaded = ValidationCache::load(&cache_dir).unwrap();

        assert_eq!(loaded.len(), 1);
        let entry = loaded.get(&file_path);
        assert!(entry.is_some());
        assert!(entry.unwrap().is_valid);
    }

    #[test]
    fn test_load_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let cache = ValidationCache::load(temp_dir.path()).unwrap();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_clear() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.yaml");
        fs::write(&file_path, "version: v1").unwrap();

        let mut cache = ValidationCache::new();
        cache.record_validation(&file_path, true).unwrap();
        assert!(!cache.is_empty());

        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_list() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("test1.yaml");
        let file2 = temp_dir.path().join("test2.yaml");
        fs::write(&file1, "version: v1").unwrap();
        fs::write(&file2, "version: v1").unwrap();

        let mut cache = ValidationCache::new();
        cache.record_validation(&file1, true).unwrap();
        cache.record_validation(&file2, false).unwrap();

        let entries: Vec<_> = cache.list().collect();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_overwrite_existing_entry() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.yaml");
        fs::write(&file_path, "version: v1").unwrap();

        let mut cache = ValidationCache::new();
        cache.record_validation(&file_path, false).unwrap(); // First: invalid
        cache.record_validation(&file_path, true).unwrap(); // Second: valid

        assert_eq!(cache.len(), 1); // Only one entry
        let entry = cache.get(&file_path).unwrap();
        assert!(entry.is_valid); // Latest value
    }

    // ==================== project_cache_dir tests ====================

    #[test]
    fn test_project_cache_dir_contains_project_name() {
        let base = PathBuf::from("/cache");
        let root = PathBuf::from("/projects/myapp");
        let dir = project_cache_dir(&base, &root);
        let name = dir.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with("myapp-"), "slug should start with directory name: {}", name);
    }

    #[test]
    fn test_project_cache_dir_slug_has_hash_suffix() {
        let base = PathBuf::from("/cache");
        let root = PathBuf::from("/projects/myapp");
        let dir = project_cache_dir(&base, &root);
        let name = dir.file_name().unwrap().to_str().unwrap();
        // Format is "<name>-<8 hex chars>"
        let parts: Vec<&str> = name.splitn(2, '-').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1].len(), 8);
        assert!(parts[1].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_project_cache_dir_same_name_different_roots_get_different_dirs() {
        let base = PathBuf::from("/cache");
        let root_a = PathBuf::from("/work/alice/myapp");
        let root_b = PathBuf::from("/work/bob/myapp");
        let dir_a = project_cache_dir(&base, &root_a);
        let dir_b = project_cache_dir(&base, &root_b);
        assert_ne!(dir_a, dir_b, "same project name, different roots must produce different dirs");
    }

    #[test]
    fn test_project_cache_dir_is_deterministic() {
        let base = PathBuf::from("/cache");
        let root = PathBuf::from("/projects/myapp");
        assert_eq!(
            project_cache_dir(&base, &root),
            project_cache_dir(&base, &root),
        );
    }

    #[test]
    fn test_project_cache_dir_nested_under_base() {
        let base = PathBuf::from("/some/cache/base");
        let root = PathBuf::from("/projects/myapp");
        let dir = project_cache_dir(&base, &root);
        assert!(dir.starts_with(&base));
        assert_eq!(dir.components().count(), base.components().count() + 1);
    }
}
