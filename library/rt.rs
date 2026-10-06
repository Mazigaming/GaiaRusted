//! Runtime support: what compiled code calls when something goes wrong.

/// `panic!` with a plain message.
pub fn panic_str(message: &'static str, location: &str) -> ! {
    std::panic::begin_panic(Box::new(message), location)
}

/// `panic!` with a formatted message.
pub fn panic_fmt(message: String, location: &str) -> ! {
    std::panic::begin_panic(Box::new(message), location)
}

pub fn panic_bounds_check(index: usize, len: usize) -> ! {
    panic_fmt(format!("index out of bounds: the len is {} but the index is {}", len, index), "<bounds check>")
}

pub fn panic_div_zero() -> ! {
    panic_str("attempt to divide by zero", "<arithmetic>")
}

pub fn assert_failed(operator: &str, left: String, right: String, location: &str) -> ! {
    panic_fmt(
        format!("assertion `left {} right` failed\n  left: {}\n right: {}", operator, left, right),
        location,
    )
}

/// String equality, for matching on string literals.
pub fn str_eq(a: &str, b: &str) -> bool {
    a == b
}
