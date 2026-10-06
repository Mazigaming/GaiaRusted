//! Input and output: the standard streams, the `Read`, `Write` and
//! `BufRead` traits that files share with them, and buffering.

use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

/// What kind of failure an I/O error is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    NotFound,
    PermissionDenied,
    AlreadyExists,
    WouldBlock,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    InvalidInput,
    InvalidData,
    TimedOut,
    WriteZero,
    Interrupted,
    Unsupported,
    UnexpectedEof,
    OutOfMemory,
    Other,
}

impl ErrorKind {
    fn description(self) -> &'static str {
        match self {
            ErrorKind::NotFound => "entity not found",
            ErrorKind::PermissionDenied => "permission denied",
            ErrorKind::AlreadyExists => "entity already exists",
            ErrorKind::WouldBlock => "operation would block",
            ErrorKind::NotADirectory => "not a directory",
            ErrorKind::IsADirectory => "is a directory",
            ErrorKind::DirectoryNotEmpty => "directory not empty",
            ErrorKind::InvalidInput => "invalid input parameter",
            ErrorKind::InvalidData => "invalid data",
            ErrorKind::TimedOut => "timed out",
            ErrorKind::WriteZero => "write zero",
            ErrorKind::Interrupted => "operation interrupted",
            ErrorKind::Unsupported => "unsupported",
            ErrorKind::UnexpectedEof => "unexpected end of file",
            ErrorKind::OutOfMemory => "out of memory",
            ErrorKind::Other => "other error",
        }
    }

    /// The kind of failure a C library error number stands for.
    fn of_errno(code: i32) -> ErrorKind {
        match code {
            1 | 13 => ErrorKind::PermissionDenied,
            2 => ErrorKind::NotFound,
            4 => ErrorKind::Interrupted,
            11 => ErrorKind::WouldBlock,
            12 => ErrorKind::OutOfMemory,
            17 => ErrorKind::AlreadyExists,
            20 => ErrorKind::NotADirectory,
            21 => ErrorKind::IsADirectory,
            22 => ErrorKind::InvalidInput,
            39 => ErrorKind::DirectoryNotEmpty,
            110 => ErrorKind::TimedOut,
            _ => ErrorKind::Other,
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.description())
    }
}

/// An I/O failure: an operating system error, or one with a message.
pub struct Error {
    repr: Repr,
}

enum Repr {
    Os(i32),
    Simple(ErrorKind),
    Custom(ErrorKind, String),
}

impl Error {
    pub fn new<M: fmt::Display>(kind: ErrorKind, message: M) -> Error {
        Error { repr: Repr::Custom(kind, message.to_string()) }
    }

    pub fn other<M: fmt::Display>(message: M) -> Error {
        Error::new(ErrorKind::Other, message)
    }

    /// The error the last failed C library call left behind.
    pub fn last_os_error() -> Error {
        Error::from_raw_os_error(unsafe { *std::libc::__errno_location() })
    }

    pub fn from_raw_os_error(code: i32) -> Error {
        Error { repr: Repr::Os(code) }
    }

    pub fn raw_os_error(&self) -> Option<i32> {
        match self.repr {
            Repr::Os(code) => Some(code),
            _ => None,
        }
    }

    pub fn kind(&self) -> ErrorKind {
        match &self.repr {
            Repr::Os(code) => ErrorKind::of_errno(*code),
            Repr::Simple(kind) => *kind,
            Repr::Custom(kind, _) => *kind,
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error { repr: Repr::Simple(kind) }
    }
}

/// The C library's description of an error number.
fn os_message(code: i32) -> String {
    unsafe {
        let text = std::libc::strerror(code);
        String::from(std::intrinsics::str_from_raw_parts(text, std::libc::strlen(text)))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.repr {
            Repr::Os(code) => write!(f, "{} (os error {})", os_message(*code), code),
            Repr::Simple(kind) => f.write_str(kind.description()),
            Repr::Custom(_, message) => f.write_str(message.as_str()),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.repr {
            Repr::Os(code) => f
                .debug_struct("Os")
                .field("code", code)
                .field("kind", &ErrorKind::of_errno(*code))
                .field("message", &os_message(*code))
                .finish(),
            Repr::Simple(kind) => f.debug_tuple("Kind").field(kind).finish(),
            Repr::Custom(kind, message) => f.debug_struct("Custom").field("kind", kind).field("error", message).finish(),
        }
    }
}

impl std::error::Error for Error {}

/// The result of a C library call that returns -1 on failure.
fn check(result: isize) -> Result<usize> {
    if result < 0 {
        Err(Error::last_os_error())
    } else {
        Ok(result as usize)
    }
}

/// `read(2)`, retried when a signal interrupts it.
pub fn read_fd(fd: i32, buffer: &mut [u8]) -> Result<usize> {
    loop {
        match check(unsafe { std::libc::read(fd, buffer.as_mut_ptr(), buffer.len()) }) {
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            done => return done,
        }
    }
}

/// `write(2)`, retried when a signal interrupts it.
pub fn write_fd(fd: i32, data: &[u8]) -> Result<usize> {
    loop {
        match check(unsafe { std::libc::write(fd, data.as_ptr(), data.len()) }) {
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            done => return done,
        }
    }
}

/// A source of bytes.
pub trait Read {
    /// Read some bytes into `buffer`, returning how many; 0 at the end.
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize>;

    /// Read everything that is left onto the end of `buffer`.
    fn read_to_end(&mut self, buffer: &mut Vec<u8>) -> Result<usize> {
        let start = buffer.len();
        let mut chunk = [0u8; 4096];
        loop {
            let count = self.read(&mut chunk)?;
            if count == 0 {
                return Ok(buffer.len() - start);
            }
            buffer.extend_from_slice(&chunk[..count]);
        }
    }

    /// Read everything that is left onto the end of `text`; it must be UTF-8.
    fn read_to_string(&mut self, text: &mut String) -> Result<usize> {
        let mut bytes = Vec::new();
        let count = self.read_to_end(&mut bytes)?;
        match std::str::from_utf8(bytes.as_slice()) {
            Ok(read) => {
                text.push_str(read);
                Ok(count)
            }
            Err(_) => Err(Error::new(ErrorKind::InvalidData, "stream did not contain valid UTF-8")),
        }
    }

    /// Fill `buffer` completely, or fail.
    fn read_exact(&mut self, buffer: &mut [u8]) -> Result<()> {
        let mut filled = 0;
        while filled < buffer.len() {
            let count = self.read(&mut buffer[filled..])?;
            if count == 0 {
                return Err(Error::new(ErrorKind::UnexpectedEof, "failed to fill whole buffer"));
            }
            filled += count;
        }
        Ok(())
    }

    /// The bytes, one at a time.
    fn bytes(self) -> Bytes<Self>
    where
        Self: Sized,
    {
        Bytes { reader: self }
    }
}

/// A destination for bytes.
pub trait Write {
    /// Write some of `data`, returning how much.
    fn write(&mut self, data: &[u8]) -> Result<usize>;

    /// Push out anything buffered.
    fn flush(&mut self) -> Result<()>;

    fn write_all(&mut self, data: &[u8]) -> Result<()> {
        let mut written = 0;
        while written < data.len() {
            let count = self.write(&data[written..])?;
            if count == 0 {
                return Err(Error::new(ErrorKind::WriteZero, "failed to write whole buffer"));
            }
            written += count;
        }
        Ok(())
    }

    /// What `write!` calls.
    fn write_fmt(&mut self, arguments: fmt::Arguments) -> Result<()> {
        self.write_all(arguments.as_str().as_bytes())
    }
}

/// A reader with a buffer, which can therefore read up to a delimiter.
pub trait BufRead: Read {
    /// The buffered bytes, reading more first if there are none.
    fn fill_buf(&mut self) -> Result<&[u8]>;

    /// Mark the first `amount` buffered bytes as used.
    fn consume(&mut self, amount: usize);

    /// Read up to and including `delimiter` onto the end of `buffer`.
    fn read_until(&mut self, delimiter: u8, buffer: &mut Vec<u8>) -> Result<usize> {
        let mut total = 0;
        loop {
            let available = self.fill_buf()?;
            if available.is_empty() {
                return Ok(total);
            }
            let (found, used) = match available.iter().position(|&byte| byte == delimiter) {
                Some(index) => (true, index + 1),
                None => (false, available.len()),
            };
            buffer.extend_from_slice(&available[..used]);
            self.consume(used);
            total += used;
            if found {
                return Ok(total);
            }
        }
    }

    /// Read a line, newline included, onto the end of `line`.
    fn read_line(&mut self, line: &mut String) -> Result<usize> {
        let mut bytes = Vec::new();
        let count = self.read_until(b'\n', &mut bytes)?;
        match std::str::from_utf8(bytes.as_slice()) {
            Ok(read) => {
                line.push_str(read);
                Ok(count)
            }
            Err(_) => Err(Error::new(ErrorKind::InvalidData, "stream did not contain valid UTF-8")),
        }
    }

    /// The lines, without their line endings.
    fn lines(self) -> Lines<Self>
    where
        Self: Sized,
    {
        Lines { reader: self }
    }

    /// The pieces between occurrences of `delimiter`.
    fn split(self, delimiter: u8) -> Split<Self>
    where
        Self: Sized,
    {
        Split { reader: self, delimiter }
    }
}

pub struct Lines<B> {
    reader: B,
}

impl<B: BufRead> Iterator for Lines<B> {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Result<String>> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) => None,
            Ok(_) => {
                if line.ends_with('\n') {
                    line.pop();
                    if line.ends_with('\r') {
                        line.pop();
                    }
                }
                Some(Ok(line))
            }
            Err(error) => Some(Err(error)),
        }
    }
}

pub struct Split<B> {
    reader: B,
    delimiter: u8,
}

impl<B: BufRead> Iterator for Split<B> {
    type Item = Result<Vec<u8>>;

    fn next(&mut self) -> Option<Result<Vec<u8>>> {
        let mut piece = Vec::new();
        match self.reader.read_until(self.delimiter, &mut piece) {
            Ok(0) => None,
            Ok(_) => {
                if piece.last() == Some(&self.delimiter) {
                    piece.pop();
                }
                Some(Ok(piece))
            }
            Err(error) => Some(Err(error)),
        }
    }
}

pub struct Bytes<R> {
    reader: R,
}

impl<R: Read> Iterator for Bytes<R> {
    type Item = Result<u8>;

    fn next(&mut self) -> Option<Result<u8>> {
        let mut byte = [0u8; 1];
        match self.reader.read(&mut byte) {
            Ok(0) => None,
            Ok(_) => Some(Ok(byte[0])),
            Err(error) => Some(Err(error)),
        }
    }
}

const STDIN: i32 = 0;
const STDOUT: i32 = 1;
const STDERR: i32 = 2;

/// Standard input read so far but not yet handed out: one buffer behind
/// every `Stdin` and lock, so nothing is lost between them.
static mut STDIN_BUFFER: [u8; 8192] = [0; 8192];
static mut STDIN_START: usize = 0;
static mut STDIN_END: usize = 0;

/// The process's standard input.
pub struct Stdin {
    _private: (),
}

/// Standard input, locked for reading line by line.
pub struct StdinLock<'a> {
    _stdin: &'a (),
}

pub fn stdin() -> Stdin {
    Stdin { _private: () }
}

impl Stdin {
    pub fn lock(&self) -> StdinLock<'static> {
        StdinLock { _stdin: &() }
    }

    pub fn read_line(&self, line: &mut String) -> Result<usize> {
        self.lock().read_line(line)
    }

    pub fn lines(self) -> Lines<StdinLock<'static>> {
        self.lock().lines()
    }
}

impl Read for Stdin {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.lock().read(buffer)
    }
}

impl<'a> Read for StdinLock<'a> {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let available = self.fill_buf()?;
        let count = available.len().min(buffer.len());
        buffer[..count].copy_from_slice(&available[..count]);
        self.consume(count);
        Ok(count)
    }
}

impl<'a> BufRead for StdinLock<'a> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        unsafe {
            if STDIN_START == STDIN_END {
                STDIN_START = 0;
                STDIN_END = read_fd(STDIN, &mut STDIN_BUFFER)?;
            }
            Ok(&STDIN_BUFFER[STDIN_START..STDIN_END])
        }
    }

    fn consume(&mut self, amount: usize) {
        unsafe {
            STDIN_START = (STDIN_START + amount).min(STDIN_END);
        }
    }
}

/// The process's standard output. Writes go straight to it.
pub struct Stdout {
    _private: (),
}

pub struct StdoutLock<'a> {
    _stdout: &'a (),
}

pub fn stdout() -> Stdout {
    Stdout { _private: () }
}

impl Stdout {
    pub fn lock(&self) -> StdoutLock<'static> {
        StdoutLock { _stdout: &() }
    }
}

impl Write for Stdout {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        write_fd(STDOUT, data)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl<'a> Write for StdoutLock<'a> {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        write_fd(STDOUT, data)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

/// The process's standard error.
pub struct Stderr {
    _private: (),
}

pub fn stderr() -> Stderr {
    Stderr { _private: () }
}

impl Stderr {
    pub fn lock(&self) -> Stderr {
        Stderr { _private: () }
    }
}

impl Write for Stderr {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        write_fd(STDERR, data)
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Reads ahead in large pieces, so that small reads and line-by-line
/// reading do not each cost a system call.
pub struct BufReader<R> {
    inner: R,
    buffer: Vec<u8>,
    start: usize,
}

impl<R: Read> BufReader<R> {
    pub fn new(inner: R) -> BufReader<R> {
        BufReader::with_capacity(8192, inner)
    }

    pub fn with_capacity(capacity: usize, inner: R) -> BufReader<R> {
        BufReader { inner, buffer: Vec::with_capacity(capacity), start: 0 }
    }

    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    pub fn into_inner(self) -> R {
        self.inner
    }

    pub fn buffer(&self) -> &[u8] {
        &self.buffer[self.start..]
    }
}

impl<R: Read> Read for BufReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        // A large read with nothing buffered goes straight through.
        if self.start == self.buffer.len() && buffer.len() >= self.buffer.capacity() {
            return self.inner.read(buffer);
        }
        let available = self.fill_buf()?;
        let count = available.len().min(buffer.len());
        buffer[..count].copy_from_slice(&available[..count]);
        self.consume(count);
        Ok(count)
    }
}

impl<R: Read> BufRead for BufReader<R> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        if self.start == self.buffer.len() {
            let capacity = self.buffer.capacity().max(1);
            self.buffer.resize(capacity, 0);
            let count = self.inner.read(self.buffer.as_mut_slice())?;
            self.buffer.truncate(count);
            self.start = 0;
        }
        Ok(&self.buffer[self.start..])
    }

    fn consume(&mut self, amount: usize) {
        self.start = (self.start + amount).min(self.buffer.len());
    }
}

/// Collects small writes and passes them on in large pieces; what is still
/// buffered is written when it is dropped.
pub struct BufWriter<W: Write> {
    inner: Option<W>,
    buffer: Vec<u8>,
}

impl<W: Write> BufWriter<W> {
    pub fn new(inner: W) -> BufWriter<W> {
        BufWriter::with_capacity(8192, inner)
    }

    pub fn with_capacity(capacity: usize, inner: W) -> BufWriter<W> {
        BufWriter { inner: Some(inner), buffer: Vec::with_capacity(capacity) }
    }

    fn flush_buffer(&mut self) -> Result<()> {
        if !self.buffer.is_empty() {
            if let Some(inner) = &mut self.inner {
                inner.write_all(self.buffer.as_slice())?;
            }
            self.buffer.clear();
        }
        Ok(())
    }

    pub fn get_ref(&self) -> &W {
        self.inner.as_ref().unwrap()
    }

    pub fn get_mut(&mut self) -> &mut W {
        self.inner.as_mut().unwrap()
    }

    /// The writer, after writing out what is buffered.
    pub fn into_inner(mut self) -> std::result::Result<W, Error> {
        self.flush_buffer()?;
        Ok(self.inner.take().unwrap())
    }

    pub fn buffer(&self) -> &[u8] {
        self.buffer.as_slice()
    }
}

impl<W: Write> Write for BufWriter<W> {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        if self.buffer.len() + data.len() > self.buffer.capacity() {
            self.flush_buffer()?;
        }
        if data.len() >= self.buffer.capacity() {
            return self.get_mut().write(data);
        }
        self.buffer.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> Result<()> {
        self.flush_buffer()?;
        self.get_mut().flush()
    }
}

impl<W: Write> Drop for BufWriter<W> {
    fn drop(&mut self) {
        let _ = self.flush_buffer();
    }
}

/// Bytes in memory read and written like a file, from a position.
pub struct Cursor<T> {
    inner: T,
    position: u64,
}

impl<T> Cursor<T> {
    pub fn new(inner: T) -> Cursor<T> {
        Cursor { inner, position: 0 }
    }

    pub fn position(&self) -> u64 {
        self.position
    }

    pub fn set_position(&mut self, position: u64) {
        self.position = position;
    }

    pub fn get_ref(&self) -> &T {
        &self.inner
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: AsRef<[u8]>> Read for Cursor<T> {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let available = self.fill_buf()?;
        let count = available.len().min(buffer.len());
        buffer[..count].copy_from_slice(&available[..count]);
        self.consume(count);
        Ok(count)
    }
}

impl<T: AsRef<[u8]>> BufRead for Cursor<T> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        let data = self.inner.as_ref();
        let start = (self.position as usize).min(data.len());
        Ok(&data[start..])
    }

    fn consume(&mut self, amount: usize) {
        self.position += amount as u64;
    }
}

impl Write for Cursor<Vec<u8>> {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        let start = self.position as usize;
        if self.inner.len() < start {
            self.inner.resize(start, 0);
        }
        let overlap = (self.inner.len() - start).min(data.len());
        self.inner[start..start + overlap].copy_from_slice(&data[..overlap]);
        self.inner.extend_from_slice(&data[overlap..]);
        self.position += data.len() as u64;
        Ok(data.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Read for &[u8] {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let count = self.len().min(buffer.len());
        buffer[..count].copy_from_slice(&self[..count]);
        *self = &self[count..];
        Ok(count)
    }
}

impl BufRead for &[u8] {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        Ok(*self)
    }

    fn consume(&mut self, amount: usize) {
        *self = &self[amount..];
    }
}

impl Write for Vec<u8> {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        self.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl<R: Read + ?Sized> Read for &mut R {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        (**self).read(buffer)
    }
}

impl<W: Write + ?Sized> Write for &mut W {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        (**self).write(data)
    }

    fn flush(&mut self) -> Result<()> {
        (**self).flush()
    }
}

impl<W: Write + ?Sized> Write for Box<W> {
    fn write(&mut self, data: &[u8]) -> Result<usize> {
        (**self).write(data)
    }

    fn flush(&mut self) -> Result<()> {
        (**self).flush()
    }
}

/// Copy everything `reader` has into `writer`.
pub fn copy<R: Read + ?Sized, W: Write + ?Sized>(reader: &mut R, writer: &mut W) -> Result<u64> {
    let mut chunk = [0u8; 8192];
    let mut total = 0u64;
    loop {
        let count = reader.read(&mut chunk)?;
        if count == 0 {
            return Ok(total);
        }
        writer.write_all(&chunk[..count])?;
        total += count as u64;
    }
}

/// Everything there is on standard input.
pub fn read_to_string<R: Read>(mut reader: R) -> Result<String> {
    let mut text = String::new();
    reader.read_to_string(&mut text)?;
    Ok(text)
}

/// Shortcuts for the common traits: `use std::io::prelude::*`.
pub mod prelude {
    pub use std::io::{BufRead, Read, Write};
}

/// Write all of `data` to a file descriptor, ignoring failures: what
/// `print!` does, which has nowhere to report them.
fn write_all_fd(fd: i32, data: &[u8]) {
    let mut written = 0;
    while written < data.len() {
        match write_fd(fd, &data[written..]) {
            Ok(count) if count > 0 => written += count,
            _ => return,
        }
    }
}

pub fn _print_str(text: &str) {
    write_all_fd(STDOUT, text.as_bytes());
}

pub fn _eprint_str(text: &str) {
    write_all_fd(STDERR, text.as_bytes());
}

pub fn _print(text: String) {
    _print_str(text.as_str());
}

pub fn _eprint(text: String) {
    _eprint_str(text.as_str());
}
