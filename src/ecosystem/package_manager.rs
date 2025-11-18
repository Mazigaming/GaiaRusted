//! Package Manager Integration
//!
//! Seamless integration with Cargo and crates.io for dependency management

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Package information
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
}

/// Dependency specification
#[derive(Debug, Clone)]
pub struct DependencySpec {
    pub name: String,
    pub version: String,
    pub source: DependencySource,
    pub features: Vec<String>,
    pub optional: bool,
}

/// Where a dependency comes from
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencySource {
    Registry,      // crates.io
    Git { url: String, branch: Option<String> },
    Path { path: PathBuf },
    Local { path: PathBuf },
}

/// Package resolution result
#[derive(Debug, Clone)]
pub struct PackageResolution {
    pub package: PackageInfo,
    pub location: PathBuf,
    pub dependencies: Vec<PackageResolution>,
    pub features_resolved: Vec<String>,
}

/// Package manager
pub struct PackageManager {
    cache_dir: PathBuf,
    registry_cache: HashMap<String, PackageInfo>,
    resolved_packages: HashMap<String, PackageResolution>,
}

impl PackageManager {
    /// Create new package manager
    pub fn new(cache_dir: PathBuf) -> Self {
        PackageManager {
            cache_dir,
            registry_cache: HashMap::new(),
            resolved_packages: HashMap::new(),
        }
    }

    /// Resolve a dependency
    pub fn resolve_dependency(
        &mut self,
        spec: &DependencySpec,
    ) -> Result<PackageResolution, String> {
        // Check if already resolved
        if let Some(resolved) = self.resolved_packages.get(&spec.name) {
            return Ok(resolved.clone());
        }

        let location = match &spec.source {
            DependencySource::Registry => {
                self.resolve_from_registry(&spec.name, &spec.version)?
            }
            DependencySource::Git { url, branch } => {
                self.resolve_from_git(url, branch.as_deref())?
            }
            DependencySource::Path { path } | DependencySource::Local { path } => {
                self.resolve_from_path(path)?
            }
        };

        let package_info = PackageInfo {
            name: spec.name.clone(),
            version: spec.version.clone(),
            authors: Vec::new(),
            description: None,
            license: None,
            repository: None,
            keywords: Vec::new(),
            categories: Vec::new(),
        };

        let resolution = PackageResolution {
            package: package_info,
            location,
            dependencies: Vec::new(),
            features_resolved: spec.features.clone(),
        };

        self.resolved_packages
            .insert(spec.name.clone(), resolution.clone());

        Ok(resolution)
    }

    /// Resolve from crates.io registry
    fn resolve_from_registry(&mut self, name: &str, version: &str) -> Result<PathBuf, String> {
        // Check cache first
        if self.registry_cache.contains_key(name) {
            let cache_path = self
                .cache_dir
                .join("registry")
                .join(format!("{}-{}", name, version));
            if cache_path.exists() {
                return Ok(cache_path);
            }
        }

        // In production, this would download from crates.io
        // For now, return a cache path
        let cache_path = self
            .cache_dir
            .join("registry")
            .join(format!("{}-{}", name, version));
        Ok(cache_path)
    }

    /// Resolve from git repository
    fn resolve_from_git(
        &mut self,
        url: &str,
        branch: Option<&str>,
    ) -> Result<PathBuf, String> {
        let branch = branch.unwrap_or("main");
        let repo_name = url
            .split('/')
            .last()
            .ok_or("Invalid git URL")?
            .trim_end_matches(".git");

        let cache_path = self
            .cache_dir
            .join("git")
            .join(format!("{}-{}", repo_name, branch));

        // In production, this would clone from git
        Ok(cache_path)
    }

    /// Resolve from local path
    fn resolve_from_path(&self, path: &Path) -> Result<PathBuf, String> {
        if !path.exists() {
            return Err(format!("Path does not exist: {}", path.display()));
        }
        Ok(path.to_path_buf())
    }

    /// Get all resolved packages
    pub fn get_resolved_packages(&self) -> Vec<&PackageResolution> {
        self.resolved_packages.values().collect()
    }

    /// Clear resolved packages
    pub fn clear_cache(&mut self) {
        self.resolved_packages.clear();
        self.registry_cache.clear();
    }

    /// Get package location
    pub fn get_package_location(&self, name: &str) -> Option<PathBuf> {
        self.resolved_packages.get(name).map(|r| r.location.clone())
    }
}

impl Default for PackageManager {
    fn default() -> Self {
        let cache_dir = dirs_home::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".gaiarusted")
            .join("cache");

        Self::new(cache_dir)
    }
}

/// Simple home directory helper (no external deps)
mod dirs_home {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .and_then(|h| {
                if h.is_empty() {
                    None::<std::ffi::OsString>
                } else {
                    Some(h)
                }
            })
            .map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_manager_creation() {
        let pm = PackageManager::new(PathBuf::from("/tmp/cache"));
        assert_eq!(pm.resolved_packages.len(), 0);
    }

    #[test]
    fn test_resolve_from_path() {
        let pm = PackageManager::new(PathBuf::from("/tmp"));
        let result = pm.resolve_from_path(Path::new("/tmp"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolve_from_path_missing() {
        let pm = PackageManager::new(PathBuf::from("/tmp"));
        let result = pm.resolve_from_path(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_dependency_spec_creation() {
        let spec = DependencySpec {
            name: "serde".to_string(),
            version: "1.0".to_string(),
            source: DependencySource::Registry,
            features: vec!["derive".to_string()],
            optional: false,
        };
        assert_eq!(spec.name, "serde");
        assert_eq!(spec.features.len(), 1);
    }

    #[test]
    fn test_package_info_creation() {
        let info = PackageInfo {
            name: "mylib".to_string(),
            version: "0.1.0".to_string(),
            authors: vec!["Alice".to_string()],
            description: Some("My library".to_string()),
            license: Some("MIT".to_string()),
            repository: None,
            keywords: vec!["testing".to_string()],
            categories: vec!["development".to_string()],
        };
        assert_eq!(info.name, "mylib");
        assert_eq!(info.keywords.len(), 1);
    }

    #[test]
    fn test_package_resolution_creation() {
        let resolution = PackageResolution {
            package: PackageInfo {
                name: "test".to_string(),
                version: "1.0".to_string(),
                authors: Vec::new(),
                description: None,
                license: None,
                repository: None,
                keywords: Vec::new(),
                categories: Vec::new(),
            },
            location: PathBuf::from("/test"),
            dependencies: Vec::new(),
            features_resolved: Vec::new(),
        };
        assert_eq!(resolution.package.name, "test");
    }

    #[test]
    fn test_dependency_source_registry() {
        let source = DependencySource::Registry;
        assert_eq!(source, DependencySource::Registry);
    }

    #[test]
    fn test_dependency_source_git() {
        let source = DependencySource::Git {
            url: "https://github.com/user/repo".to_string(),
            branch: Some("main".to_string()),
        };
        match source {
            DependencySource::Git { url, branch } => {
                assert_eq!(url, "https://github.com/user/repo");
                assert_eq!(branch, Some("main".to_string()));
            }
            _ => panic!("Expected Git source"),
        }
    }
}
