//! Community Package Registry
//!
//! Support for package registries like crates.io

use std::collections::HashMap;

/// Package metadata in registry
#[derive(Debug, Clone)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub description: String,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
    pub downloads: u64,
    pub recent_downloads: u64,
}

/// Package index entry
#[derive(Debug, Clone)]
pub struct PackageEntry {
    pub metadata: PackageMetadata,
    pub versions: Vec<String>,
    pub latest_version: String,
}

/// Package registry
pub struct PackageRegistry {
    packages: HashMap<String, PackageEntry>,
    registry_url: String,
}

impl PackageRegistry {
    /// Create new registry
    pub fn new(registry_url: String) -> Self {
        PackageRegistry {
            packages: HashMap::new(),
            registry_url,
        }
    }

    /// Search for package
    pub fn search(&self, query: &str) -> Vec<&PackageEntry> {
        self.packages
            .values()
            .filter(|entry| {
                entry.metadata.name.contains(query)
                    || entry
                        .metadata
                        .description
                        .to_lowercase()
                        .contains(&query.to_lowercase())
            })
            .collect()
    }

    /// Get package by name
    pub fn get_package(&self, name: &str) -> Option<&PackageEntry> {
        self.packages.get(name)
    }

    /// Get package version
    pub fn get_version(&self, name: &str, version: &str) -> Option<&PackageMetadata> {
        self.packages.get(name).and_then(|entry| {
            if entry.versions.contains(&version.to_string()) {
                Some(&entry.metadata)
            } else {
                None
            }
        })
    }

    /// Register package
    pub fn register_package(&mut self, metadata: PackageMetadata) -> Result<(), String> {
        if self.packages.contains_key(&metadata.name) {
            return Err(format!("Package {} already registered", metadata.name));
        }

        let latest_version = metadata.version.clone();
        let entry = PackageEntry {
            metadata,
            versions: vec![latest_version.clone()],
            latest_version,
        };

        self.packages.insert(entry.metadata.name.clone(), entry);
        Ok(())
    }

    /// Add version to package
    pub fn add_version(&mut self, name: &str, version: String) -> Result<(), String> {
        let entry = self
            .packages
            .get_mut(name)
            .ok_or_else(|| format!("Package {} not found", name))?;

        if entry.versions.contains(&version) {
            return Err(format!("Version {} already exists", version));
        }

        entry.versions.push(version.clone());
        entry.latest_version = version;
        Ok(())
    }

    /// Get registry URL
    pub fn get_registry_url(&self) -> &str {
        &self.registry_url
    }

    /// Get all packages
    pub fn list_packages(&self) -> Vec<&PackageEntry> {
        self.packages.values().collect()
    }

    /// Get packages by category
    pub fn get_by_category(&self, category: &str) -> Vec<&PackageEntry> {
        self.packages
            .values()
            .filter(|entry| entry.metadata.categories.contains(&category.to_string()))
            .collect()
    }

    /// Increment download count
    pub fn record_download(&mut self, name: &str) -> Result<(), String> {
        let entry = self
            .packages
            .get_mut(name)
            .ok_or_else(|| format!("Package {} not found", name))?;

        entry.metadata.downloads += 1;
        entry.metadata.recent_downloads += 1;
        Ok(())
    }
}

impl Default for PackageRegistry {
    fn default() -> Self {
        Self::new("https://crates.io".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = PackageRegistry::new("https://crates.io".to_string());
        assert_eq!(registry.get_registry_url(), "https://crates.io");
    }

    #[test]
    fn test_register_package() {
        let mut registry = PackageRegistry::new("https://crates.io".to_string());
        let metadata = PackageMetadata {
            name: "mylib".to_string(),
            version: "0.1.0".to_string(),
            authors: vec!["Alice".to_string()],
            description: "A nice library".to_string(),
            license: Some("MIT".to_string()),
            repository: Some("https://github.com/alice/mylib".to_string()),
            keywords: vec!["testing".to_string()],
            categories: vec!["development".to_string()],
            downloads: 0,
            recent_downloads: 0,
        };

        let result = registry.register_package(metadata);
        assert!(result.is_ok());
        assert!(registry.get_package("mylib").is_some());
    }

    #[test]
    fn test_register_duplicate_package() {
        let mut registry = PackageRegistry::new("https://crates.io".to_string());
        let metadata = PackageMetadata {
            name: "duplicate".to_string(),
            version: "1.0.0".to_string(),
            authors: vec![],
            description: "Test".to_string(),
            license: None,
            repository: None,
            keywords: vec![],
            categories: vec![],
            downloads: 0,
            recent_downloads: 0,
        };

        registry.register_package(metadata.clone()).unwrap();
        let result = registry.register_package(metadata);
        assert!(result.is_err());
    }

    #[test]
    fn test_search_package() {
        let mut registry = PackageRegistry::new("https://crates.io".to_string());
        let metadata = PackageMetadata {
            name: "searchable".to_string(),
            version: "1.0.0".to_string(),
            authors: vec![],
            description: "A searchable package".to_string(),
            license: None,
            repository: None,
            keywords: vec![],
            categories: vec![],
            downloads: 0,
            recent_downloads: 0,
        };

        registry.register_package(metadata).unwrap();
        let results = registry.search("searchable");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_get_version() {
        let mut registry = PackageRegistry::new("https://crates.io".to_string());
        let metadata = PackageMetadata {
            name: "versioned".to_string(),
            version: "1.0.0".to_string(),
            authors: vec![],
            description: "Test".to_string(),
            license: None,
            repository: None,
            keywords: vec![],
            categories: vec![],
            downloads: 0,
            recent_downloads: 0,
        };

        registry.register_package(metadata).unwrap();
        registry.add_version("versioned", "1.1.0".to_string()).unwrap();

        let version = registry.get_version("versioned", "1.0.0");
        assert!(version.is_some());
    }

    #[test]
    fn test_get_by_category() {
        let mut registry = PackageRegistry::new("https://crates.io".to_string());
        let metadata = PackageMetadata {
            name: "categorized".to_string(),
            version: "1.0.0".to_string(),
            authors: vec![],
            description: "Test".to_string(),
            license: None,
            repository: None,
            keywords: vec![],
            categories: vec!["development".to_string()],
            downloads: 0,
            recent_downloads: 0,
        };

        registry.register_package(metadata).unwrap();
        let results = registry.get_by_category("development");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_record_download() {
        let mut registry = PackageRegistry::new("https://crates.io".to_string());
        let metadata = PackageMetadata {
            name: "downloaded".to_string(),
            version: "1.0.0".to_string(),
            authors: vec![],
            description: "Test".to_string(),
            license: None,
            repository: None,
            keywords: vec![],
            categories: vec![],
            downloads: 0,
            recent_downloads: 0,
        };

        registry.register_package(metadata).unwrap();
        registry.record_download("downloaded").unwrap();

        let package = registry.get_package("downloaded").unwrap();
        assert_eq!(package.metadata.downloads, 1);
    }
}
