
use std::path::{Path, PathBuf, Component};
use std::fs;

pub struct PathUtilities;

impl PathUtilities {
    pub fn join(base: &str, path: &str) -> String {
        let p = PathBuf::from(base);
        p.join(path).to_string_lossy().to_string()
    }

    pub fn normalize(path: &str) -> String {
        let p = PathBuf::from(path);
        p.canonicalize()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string())
    }

    pub fn file_name(path: &str) -> String {
        Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    }

    pub fn extension(path: &str) -> String {
        Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string()
    }

    pub fn parent(path: &str) -> String {
        Path::new(path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string())
    }

    pub fn stem(path: &str) -> String {
        Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string()
    }

    pub fn is_absolute(path: &str) -> bool {
        Path::new(path).is_absolute()
    }

    pub fn is_relative(path: &str) -> bool {
        !Path::new(path).is_absolute()
    }

    pub fn exists(path: &str) -> bool {
        Path::new(path).exists()
    }

    pub fn is_file(path: &str) -> bool {
        Path::new(path).is_file()
    }

    pub fn is_dir(path: &str) -> bool {
        Path::new(path).is_dir()
    }

    pub fn is_symlink(path: &str) -> bool {
        Path::new(path).is_symlink()
    }

    pub fn read_dir(path: &str) -> Result<Vec<String>, String> {
        match fs::read_dir(path) {
            Ok(entries) => {
                let mut paths = Vec::new();
                for entry in entries {
                    if let Ok(entry) = entry {
                        if let Some(name) = entry.file_name().to_str() {
                            paths.push(name.to_string());
                        }
                    }
                }
                Ok(paths)
            }
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn create_dir(path: &str) -> Result<(), String> {
        fs::create_dir(path).map_err(|e| e.to_string())
    }

    pub fn create_dir_all(path: &str) -> Result<(), String> {
        fs::create_dir_all(path).map_err(|e| e.to_string())
    }

    pub fn remove_dir(path: &str) -> Result<(), String> {
        fs::remove_dir(path).map_err(|e| e.to_string())
    }

    pub fn remove_dir_all(path: &str) -> Result<(), String> {
        fs::remove_dir_all(path).map_err(|e| e.to_string())
    }

    pub fn remove_file(path: &str) -> Result<(), String> {
        fs::remove_file(path).map_err(|e| e.to_string())
    }

    pub fn rename(from: &str, to: &str) -> Result<(), String> {
        fs::rename(from, to).map_err(|e| e.to_string())
    }

    pub fn copy_file(from: &str, to: &str) -> Result<u64, String> {
        fs::copy(from, to).map_err(|e| e.to_string())
    }

    pub fn metadata(path: &str) -> Result<FileMetadata, String> {
        match fs::metadata(path) {
            Ok(meta) => Ok(FileMetadata {
                is_file: meta.is_file(),
                is_dir: meta.is_dir(),
                len: meta.len(),
                is_symlink: meta.is_symlink(),
            }),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn file_size(path: &str) -> Result<u64, String> {
        match fs::metadata(path) {
            Ok(meta) => Ok(meta.len()),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn current_dir() -> Result<String, String> {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .map_err(|e| e.to_string())
    }

    pub fn set_current_dir(path: &str) -> Result<(), String> {
        std::env::set_current_dir(path).map_err(|e| e.to_string())
    }

    pub fn canonicalize(path: &str) -> Result<String, String> {
        match fs::canonicalize(path) {
            Ok(p) => Ok(p.to_string_lossy().to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn components(path: &str) -> Vec<String> {
        Path::new(path)
            .components()
            .map(|c| match c {
                Component::Prefix(p) => p.as_os_str().to_string_lossy().to_string(),
                Component::RootDir => "/".to_string(),
                Component::CurDir => ".".to_string(),
                Component::ParentDir => "..".to_string(),
                Component::Normal(n) => n.to_string_lossy().to_string(),
            })
            .collect()
    }

    pub fn has_root(path: &str) -> bool {
        Path::new(path).has_root()
    }
}

pub struct FileMetadata {
    pub is_file: bool,
    pub is_dir: bool,
    pub len: u64,
    pub is_symlink: bool,
}

pub struct DirectoryWalker {
    root: PathBuf,
    entries: Vec<PathBuf>,
}

impl DirectoryWalker {
    pub fn new(root: &str) -> Result<Self, String> {
        let root = PathBuf::from(root);
        if !root.exists() {
            return Err("Path does not exist".to_string());
        }

        Ok(DirectoryWalker {
            root,
            entries: Vec::new(),
        })
    }

    pub fn walk_recursive(&mut self) -> Result<Vec<String>, String> {
        self.walk_recursive_internal(&self.root.clone())?;
        Ok(self.entries.iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect())
    }

    fn walk_recursive_internal(&mut self, dir: &PathBuf) -> Result<(), String> {
        match fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        self.entries.push(path.clone());

                        if path.is_dir() {
                            self.walk_recursive_internal(&path)?;
                        }
                    }
                }
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn filter_by_extension(&self, ext: &str) -> Vec<String> {
        self.entries
            .iter()
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e == ext)
                    .unwrap_or(false)
            })
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_name() {
        let name = PathUtilities::file_name("/path/to/file.txt");
        assert_eq!(name, "file.txt");
    }

    #[test]
    fn test_extension() {
        let ext = PathUtilities::extension("/path/to/file.txt");
        assert_eq!(ext, "txt");
    }

    #[test]
    fn test_parent() {
        let parent = PathUtilities::parent("/path/to/file.txt");
        assert!(parent.contains("path"));
    }

    #[test]
    fn test_stem() {
        let stem = PathUtilities::stem("/path/to/file.txt");
        assert_eq!(stem, "file");
    }

    #[test]
    fn test_is_absolute() {
        assert!(PathUtilities::is_absolute("/path/to/file"));
        assert!(!PathUtilities::is_absolute("path/to/file"));
    }

    #[test]
    fn test_is_relative() {
        assert!(!PathUtilities::is_relative("/path/to/file"));
        assert!(PathUtilities::is_relative("path/to/file"));
    }

    #[test]
    fn test_join() {
        let joined = PathUtilities::join("/path", "file.txt");
        assert!(joined.contains("file.txt"));
    }

    #[test]
    fn test_current_dir() {
        let result = PathUtilities::current_dir();
        assert!(result.is_ok());
    }

    #[test]
    fn test_components() {
        let comps = PathUtilities::components("/path/to/file.txt");
        assert!(!comps.is_empty());
    }
}
