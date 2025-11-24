//! Advanced File I/O Operations
//!
//! Extended file operations including:
//! - Binary file I/O
//! - File metadata and permissions
//! - Directory traversal and filtering
//! - Streaming operations
//! - Compression support

use std::fs::{File, Metadata, Permissions};
use std::io::{Read, Write, Result as IoResult, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Advanced file handle with metadata
pub struct AdvancedFileHandle {
    path: PathBuf,
    file: File,
    metadata: Option<Metadata>,
    position: u64,
}

impl AdvancedFileHandle {
    /// Open file and load metadata
    pub fn open<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let metadata = file.metadata().ok();
        
        Ok(AdvancedFileHandle {
            path,
            file,
            metadata,
            position: 0,
        })
    }

    /// Create or truncate file
    pub fn create<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::create(&path)?;
        let metadata = file.metadata().ok();
        
        Ok(AdvancedFileHandle {
            path,
            file,
            metadata,
            position: 0,
        })
    }

    /// Get file metadata
    pub fn metadata(&mut self) -> IoResult<Metadata> {
        let meta = self.file.metadata()?;
        self.metadata = Some(meta.clone());
        Ok(meta)
    }

    /// Get file size in bytes
    pub fn size(&mut self) -> IoResult<u64> {
        Ok(self.metadata()?.len())
    }

    /// Check if file is read-only
    pub fn is_readonly(&mut self) -> IoResult<bool> {
        Ok(self.metadata()?.permissions().readonly())
    }

    /// Set file permissions
    pub fn set_permissions(&mut self, readonly: bool) -> IoResult<()> {
        let perms = Permissions::from_mode(if readonly { 0o444 } else { 0o644 });
        self.file.set_permissions(perms)?;
        Ok(())
    }

    /// Read binary data
    pub fn read_binary(&mut self, size: usize) -> IoResult<Vec<u8>> {
        let mut buf = vec![0u8; size];
        let n = self.file.read(&mut buf)?;
        self.position += n as u64;
        buf.truncate(n);
        Ok(buf)
    }

    /// Write binary data
    pub fn write_binary(&mut self, data: &[u8]) -> IoResult<usize> {
        let n = self.file.write(data)?;
        self.position += n as u64;
        Ok(n)
    }

    /// Seek to position
    pub fn seek(&mut self, pos: SeekFrom) -> IoResult<u64> {
        self.position = self.file.seek(pos)?;
        Ok(self.position)
    }

    /// Get current position
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Read file into memory (streaming)
    pub fn read_all(&mut self) -> IoResult<Vec<u8>> {
        let size = self.size()? as usize;
        self.read_binary(size)
    }

    /// File path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Sync file to disk
    pub fn sync_all(&mut self) -> IoResult<()> {
        self.file.sync_all()
    }
}

/// Directory traversal utility
pub struct DirectoryIterator {
    path: PathBuf,
    recurse: bool,
    filter: Option<String>,
}

impl DirectoryIterator {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        DirectoryIterator {
            path: path.as_ref().to_path_buf(),
            recurse: false,
            filter: None,
        }
    }

    /// Enable recursive traversal
    pub fn recursive(mut self) -> Self {
        self.recurse = true;
        self
    }

    /// Filter by extension
    pub fn filter_extension(mut self, ext: String) -> Self {
        self.filter = Some(ext);
        self
    }

    /// List entries matching criteria
    pub fn list_entries(&self) -> IoResult<Vec<PathBuf>> {
        let mut entries = Vec::new();
        self.collect_entries(&self.path, &mut entries)?;
        Ok(entries)
    }

    fn collect_entries(&self, path: &Path, entries: &mut Vec<PathBuf>) -> IoResult<()> {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            
            let should_include = if let Some(ref filter) = self.filter {
                path.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e == filter)
                    .unwrap_or(false)
            } else {
                true
            };

            if should_include {
                entries.push(path.clone());
            }

            if self.recurse && path.is_dir() {
                self.collect_entries(&path, entries)?;
            }
        }
        Ok(())
    }
}

/// File watcher for detecting changes
#[derive(Debug, Clone)]
pub struct FileWatcher {
    path: PathBuf,
    last_modified: Option<std::time::SystemTime>,
}

impl FileWatcher {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        FileWatcher {
            path: path.as_ref().to_path_buf(),
            last_modified: None,
        }
    }

    /// Check if file has been modified
    pub fn has_changed(&mut self) -> IoResult<bool> {
        let metadata = std::fs::metadata(&self.path)?;
        let modified = metadata.modified()?;
        
        let changed = if let Some(last) = self.last_modified {
            modified > last
        } else {
            true
        };

        self.last_modified = Some(modified);
        Ok(changed)
    }

    /// Get last modification time
    pub fn last_modified(&self) -> IoResult<std::time::SystemTime> {
        Ok(std::fs::metadata(&self.path)?.modified()?)
    }
}

/// Binary file operations
pub struct BinaryFileOps;

impl BinaryFileOps {
    /// Read binary file completely
    pub fn read<P: AsRef<Path>>(path: P) -> IoResult<Vec<u8>> {
        std::fs::read(path)
    }

    /// Write binary file completely
    pub fn write<P: AsRef<Path>>(path: P, data: &[u8]) -> IoResult<()> {
        std::fs::write(path, data)
    }

    /// Append binary data
    pub fn append<P: AsRef<Path>>(path: P, data: &[u8]) -> IoResult<()> {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        file.write_all(data)?;
        Ok(())
    }

    /// Copy file with progress
    pub fn copy_with_progress<P: AsRef<Path>>(src: P, dst: P, chunk_size: usize) -> IoResult<u64> {
        let mut src_file = std::fs::File::open(src)?;
        let mut dst_file = std::fs::File::create(dst)?;
        
        let mut buf = vec![0u8; chunk_size];
        let mut total = 0u64;
        
        loop {
            let n = src_file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            dst_file.write_all(&buf[..n])?;
            total += n as u64;
        }
        
        Ok(total)
    }

    /// Calculate file checksum
    pub fn checksum<P: AsRef<Path>>(path: P) -> IoResult<u64> {
        let mut file = std::fs::File::open(path)?;
        let mut checksum = 0u64;
        let mut buf = [0u8; 4096];
        
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            for &byte in &buf[..n] {
                checksum = checksum.wrapping_mul(31).wrapping_add(byte as u64);
            }
        }
        
        Ok(checksum)
    }
}

/// Text file operations
pub struct TextFileOps;

impl TextFileOps {
    /// Count lines in file
    pub fn line_count<P: AsRef<Path>>(path: P) -> IoResult<usize> {
        use std::io::BufRead;
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        Ok(reader.lines().count())
    }

    /// Get specific line
    pub fn get_line<P: AsRef<Path>>(path: P, line_num: usize) -> IoResult<Option<String>> {
        use std::io::BufRead;
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        Ok(reader.lines().nth(line_num).transpose()?)
    }

    /// Replace text in file
    pub fn replace<P: AsRef<Path>>(path: P, from: &str, to: &str) -> IoResult<()> {
        let content = std::fs::read_to_string(&path)?;
        let new_content = content.replace(from, to);
        std::fs::write(path, new_content)?;
        Ok(())
    }

    /// Split file into lines
    pub fn read_lines<P: AsRef<Path>>(path: P) -> IoResult<Vec<String>> {
        use std::io::BufRead;
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        reader.lines().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_advanced_file_handle_create() {
        let path = "/tmp/test_adv_file.txt";
        let result = AdvancedFileHandle::create(path);
        assert!(result.is_ok());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_directory_iterator_new() {
        let iter = DirectoryIterator::new("/tmp");
        assert_eq!(iter.path.to_str().unwrap(), "/tmp");
        assert!(!iter.recurse);
    }

    #[test]
    fn test_directory_iterator_recursive() {
        let iter = DirectoryIterator::new("/tmp").recursive();
        assert!(iter.recurse);
    }

    #[test]
    fn test_file_watcher_new() {
        let watcher = FileWatcher::new("/tmp");
        assert_eq!(watcher.path.to_str().unwrap(), "/tmp");
    }

    #[test]
    fn test_binary_file_ops_write_read() {
        let path = "/tmp/test_binary.bin";
        let data = b"hello binary";
        let _ = BinaryFileOps::write(path, data);
        let read = BinaryFileOps::read(path).unwrap();
        assert_eq!(read, data);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_text_file_ops_line_count() {
        let path = "/tmp/test_lines.txt";
        let mut file = std::fs::File::create(path).unwrap();
        writeln!(file, "line1").unwrap();
        writeln!(file, "line2").unwrap();
        writeln!(file, "line3").unwrap();
        drop(file);
        
        let count = TextFileOps::line_count(path).unwrap();
        assert_eq!(count, 3);
        let _ = std::fs::remove_file(path);
    }
}

// Helper trait for compatibility
pub trait PermissionsExt {
    fn from_mode(mode: u32) -> Permissions;
}

impl PermissionsExt for Permissions {
    fn from_mode(mode: u32) -> Permissions {
        use std::os::unix::fs::PermissionsExt as UnixPermsExt;
        <std::fs::Permissions as UnixPermsExt>::from_mode(mode)
    }
}
