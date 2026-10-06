//! The running process.

/// End the process now with `code` as its exit status. Destructors of
/// values still alive do not run.
pub fn exit(code: i32) -> ! {
    unsafe { std::libc::exit(code) }
}

/// End the process abnormally, as on a fatal error.
pub fn abort() -> ! {
    unsafe { std::libc::abort() }
}

/// The process's identifier.
pub fn id() -> u32 {
    unsafe { std::libc::getpid() as u32 }
}
