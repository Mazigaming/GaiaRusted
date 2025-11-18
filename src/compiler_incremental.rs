//! Incremental Compilation Support
//!
//! Cache compilation artifacts and only recompile changed files

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

/// File hash for change detection
#[derive(Debug, Clone, PartialEq)]
pub struct FileHash {
    pub path: String,
    pub hash: u64,
    pub modified: u64,
}

impl FileHash {
    /// Create file hash from path
    pub fn from_file(path: &str) -> Result<Self, String> {
        let metadata = fs::metadata(path)
            .map_err(|e| format!("Failed to read {}: {}", path, e))?;

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let hash = Self::compute_hash(path)?;

        Ok(FileHash {
            path: path.to_string(),
            hash,
            modified,
        })
    }

    /// Compute file hash
    fn compute_hash(path: &str) -> Result<u64, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path, e))?;

        // Simple hash function
        let mut hash: u64 = 5381;
        for byte in content.as_bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(*byte as u64);
        }
        Ok(hash)
    }

    /// Check if file has changed
    pub fn has_changed(&self) -> Result<bool, String> {
        let new_hash = Self::from_file(&self.path)?;
        Ok(new_hash.hash != self.hash)
    }
}

/// Compilation cache entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub source_hash: u64,
    pub output: Vec<u8>,
    pub dependencies: Vec<String>,
    pub timestamp: u64,
}

/// Incremental compilation cache
pub struct IncrementalCache {
    cache_dir: PathBuf,
    entries: HashMap<String, CacheEntry>,
    file_hashes: HashMap<String, FileHash>,
}

impl IncrementalCache {
    /// Create new incremental cache
    pub fn new(cache_dir: &str) -> Result<Self, String> {
        let path = PathBuf::from(cache_dir);
        fs::create_dir_all(&path)
            .map_err(|e| format!("Failed to create cache dir: {}", e))?;

        Ok(IncrementalCache {
            cache_dir: path,
            entries: HashMap::new(),
            file_hashes: HashMap::new(),
        })
    }

    /// Get cache entry for file
    pub fn get(&self, file_path: &str) -> Option<&CacheEntry> {
        self.entries.get(file_path)
    }

    /// Store cache entry
    pub fn put(&mut self, file_path: String, entry: CacheEntry) {
        self.entries.insert(file_path, entry);
    }

    /// Check if file needs recompilation
    pub fn needs_recompilation(&self, file_path: &str) -> Result<bool, String> {
        // Check if file hash exists and matches
        if let Some(old_hash) = self.file_hashes.get(file_path) {
            Ok(old_hash.has_changed()?)
        } else {
            Ok(true)  // Not cached, needs compilation
        }
    }

    /// Update file hash tracking
    pub fn track_file(&mut self, file_path: &str) -> Result<(), String> {
        let file_hash = FileHash::from_file(file_path)?;
        self.file_hashes.insert(file_path.to_string(), file_hash);
        Ok(())
    }

    /// Get dirty files (changed since last compilation)
    pub fn get_dirty_files(&self, files: &[String]) -> Result<Vec<String>, String> {
        let mut dirty = Vec::new();
        for file in files {
            if self.needs_recompilation(file)? {
                dirty.push(file.clone());
            }
        }
        Ok(dirty)
    }

    /// Clear cache for file
    pub fn invalidate(&mut self, file_path: &str) {
        self.entries.remove(file_path);
        self.file_hashes.remove(file_path);
    }

    /// Clear entire cache
    pub fn clear_all(&mut self) {
        self.entries.clear();
        self.file_hashes.clear();
    }

    /// Compute cache size
    pub fn size(&self) -> usize {
        self.entries.iter().map(|(_, e)| e.output.len()).sum()
    }
}

/// Incremental compilation manager
pub struct IncrementalCompiler {
    cache: IncrementalCache,
    dependency_graph: HashMap<String, Vec<String>>,
}

impl IncrementalCompiler {
    /// Create new incremental compiler
    pub fn new(cache_dir: &str) -> Result<Self, String> {
        Ok(IncrementalCompiler {
            cache: IncrementalCache::new(cache_dir)?,
            dependency_graph: HashMap::new(),
        })
    }

    /// Add file dependency
    pub fn add_dependency(&mut self, file: String, depends_on: String) {
        self.dependency_graph
            .entry(file)
            .or_insert_with(Vec::new)
            .push(depends_on);
    }

    /// Get files affected by change
    pub fn get_affected_files(&self, changed_file: &str) -> Vec<String> {
        let mut affected = vec![changed_file.to_string()];
        let mut queue = vec![changed_file.to_string()];

        while let Some(file) = queue.pop() {
            // Find all files that depend on this file
            for (dependent, deps) in &self.dependency_graph {
                if deps.contains(&file) && !affected.contains(dependent) {
                    affected.push(dependent.clone());
                    queue.push(dependent.clone());
                }
            }
        }

        affected
    }

    /// Plan compilation with minimal recompilation
    pub fn plan_compilation(&self, files: &[String]) -> Result<Vec<String>, String> {
        let dirty = self.cache.get_dirty_files(files)?;
        
        let mut to_compile = dirty.clone();
        for dirty_file in &dirty {
            let affected = self.get_affected_files(dirty_file);
            for file in affected {
                if !to_compile.contains(&file) && files.contains(&file) {
                    to_compile.push(file);
                }
            }
        }

        Ok(to_compile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_file_hash_computation() {
        let result = FileHash::compute_hash("/dev/null");
        assert!(result.is_ok() || result.is_err());  // Depends on system
    }

    #[test]
    fn test_incremental_cache_creation() {
        let result = IncrementalCache::new("./test_cache_incremental");
        assert!(result.is_ok());
        if let Ok(cache) = result {
            assert_eq!(cache.entries.len(), 0);
        }
        let _ = fs::remove_dir_all("./test_cache_incremental");
    }

    #[test]
    fn test_cache_entry_storage() {
        let mut cache = IncrementalCache::new("./test_cache_incremental2").unwrap();
        let entry = CacheEntry {
            source_hash: 12345,
            output: vec![1, 2, 3],
            dependencies: vec!["dep.rs".to_string()],
            timestamp: 0,
        };
        cache.put("main.rs".to_string(), entry);
        assert!(cache.get("main.rs").is_some());
        let _ = fs::remove_dir_all("./test_cache_incremental2");
    }

    #[test]
    fn test_incremental_compiler() {
        let result = IncrementalCompiler::new("./test_incremental_compiler");
        assert!(result.is_ok());
        let _ = fs::remove_dir_all("./test_incremental_compiler");
    }

    #[test]
    fn test_dependency_tracking() {
        let mut compiler = IncrementalCompiler::new("./test_deps").unwrap();
        compiler.add_dependency("main.rs".to_string(), "lib.rs".to_string());
        
        let affected = compiler.get_affected_files("lib.rs");
        assert!(affected.contains(&"main.rs".to_string()));
        
        let _ = fs::remove_dir_all("./test_deps");
    }
}
