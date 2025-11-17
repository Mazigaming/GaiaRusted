use std::fs::{File, OpenOptions, metadata, create_dir, remove_file, remove_dir};
use std::io::{Read, Write, BufReader, BufWriter, Result as IoResult};
use std::path::Path;
use std::collections::HashMap;

pub struct FileHandle {
    file: File,
    path: String,
    mode: FileMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMode {
    Read,
    Write,
    Append,
    ReadWrite,
}

impl FileHandle {
    pub fn open<P: AsRef<Path>>(path: P, mode: FileMode) -> IoResult<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let file = match mode {
            FileMode::Read => File::open(path)?,
            FileMode::Write => File::create(path)?,
            FileMode::Append => OpenOptions::new().append(true).open(path)?,
            FileMode::ReadWrite => {
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(path)?
            }
        };

        Ok(FileHandle {
            file,
            path: path_str,
            mode,
        })
    }

    pub fn read_to_string(&mut self) -> IoResult<String> {
        let mut contents = String::new();
        self.file.read_to_string(&mut contents)?;
        Ok(contents)
    }

    pub fn read_line(&mut self) -> IoResult<String> {
        use std::io::BufRead;
        let mut reader = BufReader::new(&self.file);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        Ok(line)
    }

    pub fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
        self.file.write_all(buf)
    }

    pub fn write_string(&mut self, content: &str) -> IoResult<()> {
        self.file.write_all(content.as_bytes())
    }

    pub fn flush(&mut self) -> IoResult<()> {
        self.file.flush()
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn mode(&self) -> FileMode {
        self.mode
    }

    pub fn seek(&mut self, pos: std::io::SeekFrom) -> IoResult<u64> {
        use std::io::Seek;
        self.file.seek(pos)
    }

    pub fn set_position(&mut self, pos: u64) -> IoResult<()> {
        use std::io::Seek;
        self.file.seek(std::io::SeekFrom::Start(pos))?;
        Ok(())
    }
}

pub struct FileSystem {
    open_files: HashMap<String, FileMode>,
}

impl FileSystem {
    pub fn new() -> Self {
        FileSystem {
            open_files: HashMap::new(),
        }
    }

    pub fn read_file<P: AsRef<Path>>(path: P) -> IoResult<String> {
        std::fs::read_to_string(path)
    }

    pub fn write_file<P: AsRef<Path>>(path: P, contents: &str) -> IoResult<()> {
        std::fs::write(path, contents)
    }

    pub fn append_file<P: AsRef<Path>>(path: P, contents: &str) -> IoResult<()> {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)?;
        file.write_all(contents.as_bytes())?;
        Ok(())
    }

    pub fn file_exists<P: AsRef<Path>>(path: P) -> bool {
        path.as_ref().exists()
    }

    pub fn is_file<P: AsRef<Path>>(path: P) -> IoResult<bool> {
        let meta = metadata(path)?;
        Ok(meta.is_file())
    }

    pub fn is_directory<P: AsRef<Path>>(path: P) -> IoResult<bool> {
        let meta = metadata(path)?;
        Ok(meta.is_dir())
    }

    pub fn file_size<P: AsRef<Path>>(path: P) -> IoResult<u64> {
        let meta = metadata(path)?;
        Ok(meta.len())
    }

    pub fn list_directory<P: AsRef<Path>>(path: P) -> IoResult<std::vec::Vec<String>> {
        let mut entries = std::vec::Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(name) = path.file_name() {
                if let Some(name_str) = name.to_str() {
                    entries.push(name_str.to_string());
                }
            }
        }
        Ok(entries)
    }

    pub fn create_directory<P: AsRef<Path>>(path: P) -> IoResult<()> {
        create_dir(path)?;
        Ok(())
    }

    pub fn create_directory_all<P: AsRef<Path>>(path: P) -> IoResult<()> {
        std::fs::create_dir_all(path)?;
        Ok(())
    }

    pub fn delete_file<P: AsRef<Path>>(path: P) -> IoResult<()> {
        remove_file(path)?;
        Ok(())
    }

    pub fn delete_directory<P: AsRef<Path>>(path: P) -> IoResult<()> {
        remove_dir(path)?;
        Ok(())
    }

    pub fn delete_directory_all<P: AsRef<Path>>(path: P) -> IoResult<()> {
        std::fs::remove_dir_all(path)?;
        Ok(())
    }

    pub fn record_file_open(&mut self, path: String, mode: FileMode) {
        self.open_files.insert(path, mode);
    }

    pub fn record_file_close(&mut self, path: &str) {
        self.open_files.remove(path);
    }

    pub fn is_file_open(&self, path: &str) -> bool {
        self.open_files.contains_key(path)
    }

    pub fn open_files_count(&self) -> usize {
        self.open_files.len()
    }
}

pub struct BufferedReader {
    reader: BufReader<File>,
    path: String,
}

impl BufferedReader {
    pub fn new<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(BufferedReader {
            reader,
            path: path_str,
        })
    }

    pub fn read_line(&mut self) -> IoResult<String> {
        use std::io::BufRead;
        let mut line = String::new();
        self.reader.read_line(&mut line)?;
        if line.is_empty() {
            Ok(String::new())
        } else {
            Ok(line)
        }
    }

    pub fn lines(&mut self) -> IoResult<std::vec::Vec<String>> {
        use std::io::BufRead;
        let mut lines = std::vec::Vec::new();
        let reader = &mut self.reader;
        for line in reader.lines() {
            lines.push(line?);
        }
        Ok(lines)
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

pub struct BufferedWriter {
    writer: BufWriter<File>,
    path: String,
}

impl BufferedWriter {
    pub fn new<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        Ok(BufferedWriter {
            writer,
            path: path_str,
        })
    }

    pub fn write(&mut self, buf: &[u8]) -> IoResult<()> {
        self.writer.write_all(buf)?;
        Ok(())
    }

    pub fn write_string(&mut self, s: &str) -> IoResult<()> {
        self.writer.write_all(s.as_bytes())?;
        Ok(())
    }

    pub fn write_line(&mut self, s: &str) -> IoResult<()> {
        self.writer.write_all(s.as_bytes())?;
        self.writer.write_all(b"\n")?;
        Ok(())
    }

    pub fn flush(&mut self) -> IoResult<()> {
        self.writer.flush()?;
        Ok(())
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

pub mod async_io {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    pub struct AsyncFileRead {
        path: String,
        data: Option<String>,
    }

    impl AsyncFileRead {
        pub fn new(path: String) -> Self {
            AsyncFileRead { path, data: None }
        }
    }

    impl Future for AsyncFileRead {
        type Output = std::io::Result<String>;

        fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.data.is_none() {
                match std::fs::read_to_string(&self.path) {
                    Ok(content) => {
                        self.data = Some(content.clone());
                        Poll::Ready(Ok(content))
                    }
                    Err(e) => Poll::Ready(Err(e)),
                }
            } else {
                Poll::Ready(Ok(self.data.take().unwrap()))
            }
        }
    }

    pub struct AsyncFileWrite {
        path: String,
        content: String,
        completed: bool,
    }

    impl AsyncFileWrite {
        pub fn new(path: String, content: String) -> Self {
            AsyncFileWrite {
                path,
                content,
                completed: false,
            }
        }
    }

    impl Future for AsyncFileWrite {
        type Output = std::io::Result<()>;

        fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            if !self.completed {
                self.completed = true;
                Poll::Ready(std::fs::write(&self.path, &self.content))
            } else {
                Poll::Ready(Ok(()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_system_new() {
        let fs = FileSystem::new();
        assert_eq!(fs.open_files_count(), 0);
    }

    #[test]
    fn test_file_mode_equality() {
        assert_eq!(FileMode::Read, FileMode::Read);
        assert_ne!(FileMode::Read, FileMode::Write);
    }

    #[test]
    fn test_file_system_read_write() {
        let test_file = "test_io.txt";
        let content = "Hello, World!";

        let result = FileSystem::write_file(test_file, content);
        assert!(result.is_ok());

        let read_result = FileSystem::read_file(test_file);
        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap(), content);

        let _ = FileSystem::delete_file(test_file);
    }

    #[test]
    fn test_file_system_file_exists() {
        let test_file = "test_exists.txt";
        FileSystem::write_file(test_file, "test").ok();
        assert!(FileSystem::file_exists(test_file));
        FileSystem::delete_file(test_file).ok();
    }

    #[test]
    fn test_file_system_append() {
        let test_file = "test_append.txt";
        FileSystem::write_file(test_file, "line1\n").ok();
        FileSystem::append_file(test_file, "line2\n").ok();

        let content = FileSystem::read_file(test_file).unwrap();
        assert!(content.contains("line1"));
        assert!(content.contains("line2"));

        FileSystem::delete_file(test_file).ok();
    }

    #[test]
    fn test_file_mode_debug() {
        let mode = FileMode::Read;
        assert_eq!(format!("{:?}", mode), "Read");
    }

    #[test]
    fn test_file_system_is_file() {
        let test_file = "test_is_file.txt";
        FileSystem::write_file(test_file, "content").ok();
        let is_file = FileSystem::is_file(test_file);
        assert!(is_file.is_ok());
        assert!(is_file.unwrap());
        FileSystem::delete_file(test_file).ok();
    }

    #[test]
    fn test_file_system_record_operations() {
        let mut fs = FileSystem::new();
        assert_eq!(fs.open_files_count(), 0);

        fs.record_file_open("test.txt".to_string(), FileMode::Read);
        assert!(fs.is_file_open("test.txt"));
        assert_eq!(fs.open_files_count(), 1);

        fs.record_file_close("test.txt");
        assert!(!fs.is_file_open("test.txt"));
        assert_eq!(fs.open_files_count(), 0);
    }

    #[test]
    fn test_buffered_writer_creation() {
        let test_file = "test_buffered_write.txt";
        let result = BufferedWriter::new(test_file);
        assert!(result.is_ok());
        let _ = FileSystem::delete_file(test_file);
    }

    #[test]
    fn test_buffered_reader_creation() {
        let test_file = "test_buffered_read.txt";
        FileSystem::write_file(test_file, "test content").ok();
        let result = BufferedReader::new(test_file);
        assert!(result.is_ok());
        FileSystem::delete_file(test_file).ok();
    }

    #[test]
    fn test_file_size() {
        let test_file = "test_size.txt";
        let content = "Hello";
        FileSystem::write_file(test_file, content).ok();

        let size_result = FileSystem::file_size(test_file);
        assert!(size_result.is_ok());
        assert!(size_result.unwrap() > 0);

        FileSystem::delete_file(test_file).ok();
    }

    #[test]
    fn test_create_and_delete_directory() {
        let dir = "test_dir_io";
        let create_result = FileSystem::create_directory(dir);
        if create_result.is_ok() {
            assert!(FileSystem::is_directory(dir).unwrap_or(false));
            FileSystem::delete_directory(dir).ok();
        }
    }

    #[test]
    fn test_buffered_writer_write_line() {
        let test_file = "test_write_line.txt";
        let result = BufferedWriter::new(test_file);
        if let Ok(mut writer) = result {
            writer.write_line("Test line").ok();
            writer.flush().ok();
        }
        FileSystem::delete_file(test_file).ok();
    }

    #[test]
    fn test_file_handle_creation() {
        let test_file = "test_handle.txt";
        FileSystem::write_file(test_file, "test").ok();

        let result = FileHandle::open(test_file, FileMode::Read);
        assert!(result.is_ok());

        FileSystem::delete_file(test_file).ok();
    }

    #[test]
    fn test_file_handle_write() {
        let test_file = "test_handle_write.txt";
        let result = FileHandle::open(test_file, FileMode::Write);
        if let Ok(mut handle) = result {
            handle.write_string("test content").ok();
            handle.flush().ok();
        }
        FileSystem::delete_file(test_file).ok();
    }

    #[test]
    fn test_file_handle_path() {
        let test_file = "test_handle_path.txt";
        FileSystem::write_file(test_file, "content").ok();
        let handle = FileHandle::open(test_file, FileMode::Read);
        if let Ok(h) = handle {
            assert_eq!(h.path(), test_file);
        }
        FileSystem::delete_file(test_file).ok();
    }

    #[test]
    fn test_async_file_operations() {
        let test_file = "test_async_ops.txt";
        FileSystem::write_file(test_file, "async content").ok();
        
        let _async_read = async_io::AsyncFileRead::new(test_file.to_string());
        let _async_write = async_io::AsyncFileWrite::new(test_file.to_string(), "new content".to_string());

        assert!(FileSystem::file_exists(test_file));

        FileSystem::delete_file(test_file).ok();
    }
}
