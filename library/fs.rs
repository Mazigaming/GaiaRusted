//! Files and the file system.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

/// A path as a C string.
fn c_path(path: &Path) -> Vec<u8> {
    let text = path.to_str().unwrap_or("");
    let mut bytes = Vec::with_capacity(text.len() + 1);
    bytes.extend_from_slice(text.as_bytes());
    bytes.push(0);
    bytes
}

/// The result of a C library call that returns -1 on failure.
fn check(result: i32) -> io::Result<i32> {
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(result)
    }
}

/// An open file, closed when dropped.
pub struct File {
    fd: i32,
}

impl File {
    /// Open an existing file for reading.
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<File> {
        OpenOptions::new().read(true).open(path)
    }

    /// Create a file for writing, emptying it if it exists.
    pub fn create<P: AsRef<Path>>(path: P) -> io::Result<File> {
        OpenOptions::new().write(true).create(true).truncate(true).open(path)
    }

    pub fn options() -> OpenOptions {
        OpenOptions::new()
    }

    pub fn metadata(&self) -> io::Result<Metadata> {
        let mut status = [0u8; STAT_SIZE];
        check(unsafe { std::libc::fstat(self.fd, status.as_mut_ptr()) })?;
        Ok(Metadata::from_stat(&status))
    }

    pub fn sync_all(&self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for File {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        io::read_fd(self.fd, buffer)
    }
}

impl Write for File {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        io::write_fd(self.fd, data)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for File {
    fn drop(&mut self) {
        unsafe {
            std::libc::close(self.fd);
        }
    }
}

impl std::fmt::Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("File").field("fd", &self.fd).finish()
    }
}

/// How to open a file: for reading, writing, appending, creating...
#[derive(Clone, Debug)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    create_new: bool,
}

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions { read: false, write: false, append: false, truncate: false, create: false, create_new: false }
    }

    pub fn read(&mut self, read: bool) -> &mut OpenOptions {
        self.read = read;
        self
    }

    pub fn write(&mut self, write: bool) -> &mut OpenOptions {
        self.write = write;
        self
    }

    pub fn append(&mut self, append: bool) -> &mut OpenOptions {
        self.append = append;
        self
    }

    pub fn truncate(&mut self, truncate: bool) -> &mut OpenOptions {
        self.truncate = truncate;
        self
    }

    pub fn create(&mut self, create: bool) -> &mut OpenOptions {
        self.create = create;
        self
    }

    pub fn create_new(&mut self, create_new: bool) -> &mut OpenOptions {
        self.create_new = create_new;
        self
    }

    pub fn open<P: AsRef<Path>>(&self, path: P) -> io::Result<File> {
        const WRITE_ONLY: i32 = 0o1;
        const READ_WRITE: i32 = 0o2;
        const CREATE: i32 = 0o100;
        const EXCLUSIVE: i32 = 0o200;
        const TRUNCATE: i32 = 0o1000;
        const APPEND: i32 = 0o2000;
        let writing = self.write || self.append;
        let mut flags = match (self.read, writing) {
            (true, true) => READ_WRITE,
            (false, true) => WRITE_ONLY,
            _ => 0,
        };
        if self.append {
            flags |= APPEND;
        }
        if self.truncate {
            flags |= TRUNCATE;
        }
        if self.create_new {
            flags |= CREATE | EXCLUSIVE;
        } else if self.create {
            flags |= CREATE;
        }
        let path = c_path(path.as_ref());
        let fd = check(unsafe { std::libc::open(path.as_ptr(), flags, 0o666) })?;
        Ok(File { fd })
    }
}

/// The size of `struct stat` on x86-64 Linux, and where in it the fields
/// read here are.
const STAT_SIZE: usize = 144;
const STAT_MODE: usize = 24;
const STAT_SIZE_FIELD: usize = 48;

/// What the file system knows about a file.
#[derive(Clone, Debug)]
pub struct Metadata {
    len: u64,
    mode: u32,
}

impl Metadata {
    fn from_stat(status: &[u8; STAT_SIZE]) -> Metadata {
        let mut mode = [0u8; 4];
        mode.copy_from_slice(&status[STAT_MODE..STAT_MODE + 4]);
        let mut len = [0u8; 8];
        len.copy_from_slice(&status[STAT_SIZE_FIELD..STAT_SIZE_FIELD + 8]);
        Metadata { len: u64::from_le_bytes(len), mode: u32::from_le_bytes(mode) }
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn is_file(&self) -> bool {
        self.mode & 0o170000 == 0o100000
    }

    pub fn is_dir(&self) -> bool {
        self.mode & 0o170000 == 0o040000
    }

    pub fn is_symlink(&self) -> bool {
        self.mode & 0o170000 == 0o120000
    }
}

pub fn metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    let path = c_path(path.as_ref());
    let mut status = [0u8; STAT_SIZE];
    check(unsafe { std::libc::stat(path.as_ptr(), status.as_mut_ptr()) })?;
    Ok(Metadata::from_stat(&status))
}

pub fn exists<P: AsRef<Path>>(path: P) -> io::Result<bool> {
    Ok(metadata(path).is_ok())
}

/// The whole contents of a file.
pub fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// The whole contents of a file, which must be UTF-8.
pub fn read_to_string<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let mut text = String::new();
    File::open(path)?.read_to_string(&mut text)?;
    Ok(text)
}

/// Make `contents` the whole contents of a file, creating it if needed.
pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    File::create(path)?.write_all(contents.as_ref())
}

pub fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = c_path(path.as_ref());
    check(unsafe { std::libc::unlink(path.as_ptr()) })?;
    Ok(())
}

pub fn create_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = c_path(path.as_ref());
    check(unsafe { std::libc::mkdir(path.as_ptr(), 0o777) })?;
    Ok(())
}

/// Create a directory and any missing parents.
pub fn create_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref();
    if path.as_os_str().is_empty() || path.is_dir() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }
    match create_dir(path) {
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists && path.is_dir() => Ok(()),
        done => done,
    }
}

pub fn remove_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = c_path(path.as_ref());
    check(unsafe { std::libc::rmdir(path.as_ptr()) })?;
    Ok(())
}

pub fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
    let (from, to) = (c_path(from.as_ref()), c_path(to.as_ref()));
    check(unsafe { std::libc::rename(from.as_ptr(), to.as_ptr()) })?;
    Ok(())
}

/// Copy a file's contents; returns how many bytes.
pub fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<u64> {
    let mut source = File::open(from)?;
    let mut target = File::create(to)?;
    io::copy(&mut source, &mut target)
}

pub fn canonicalize<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    let path = path.as_ref();
    metadata(path)?;
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}
