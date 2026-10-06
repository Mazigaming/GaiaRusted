//! Spans of time, and points in time to measure them with.

use std::fmt;
use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};

const NANOS_PER_SEC: u32 = 1_000_000_000;

/// A span of time, to the nanosecond.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Duration {
    secs: u64,
    nanos: u32,
}

impl Duration {
    pub const ZERO: Duration = Duration { secs: 0, nanos: 0 };
    pub const MAX: Duration = Duration { secs: u64::MAX, nanos: 999_999_999 };
    pub const SECOND: Duration = Duration { secs: 1, nanos: 0 };
    pub const MILLISECOND: Duration = Duration { secs: 0, nanos: 1_000_000 };
    pub const MICROSECOND: Duration = Duration { secs: 0, nanos: 1_000 };
    pub const NANOSECOND: Duration = Duration { secs: 0, nanos: 1 };

    pub fn new(secs: u64, nanos: u32) -> Duration {
        let extra = (nanos / NANOS_PER_SEC) as u64;
        Duration { secs: secs + extra, nanos: nanos % NANOS_PER_SEC }
    }

    pub fn from_secs(secs: u64) -> Duration {
        Duration { secs, nanos: 0 }
    }

    pub fn from_millis(millis: u64) -> Duration {
        Duration { secs: millis / 1000, nanos: (millis % 1000) as u32 * 1_000_000 }
    }

    pub fn from_micros(micros: u64) -> Duration {
        Duration { secs: micros / 1_000_000, nanos: (micros % 1_000_000) as u32 * 1000 }
    }

    pub fn from_nanos(nanos: u64) -> Duration {
        Duration { secs: nanos / NANOS_PER_SEC as u64, nanos: (nanos % NANOS_PER_SEC as u64) as u32 }
    }

    pub fn from_secs_f64(secs: f64) -> Duration {
        let whole = secs.trunc();
        Duration::new(whole as u64, ((secs - whole) * NANOS_PER_SEC as f64).round() as u32)
    }

    pub fn from_secs_f32(secs: f32) -> Duration {
        Duration::from_secs_f64(secs as f64)
    }

    pub fn is_zero(&self) -> bool {
        self.secs == 0 && self.nanos == 0
    }

    pub fn as_secs(&self) -> u64 {
        self.secs
    }

    pub fn subsec_millis(&self) -> u32 {
        self.nanos / 1_000_000
    }

    pub fn subsec_micros(&self) -> u32 {
        self.nanos / 1000
    }

    pub fn subsec_nanos(&self) -> u32 {
        self.nanos
    }

    pub fn as_millis(&self) -> u128 {
        self.secs as u128 * 1000 + (self.nanos / 1_000_000) as u128
    }

    pub fn as_micros(&self) -> u128 {
        self.secs as u128 * 1_000_000 + (self.nanos / 1000) as u128
    }

    pub fn as_nanos(&self) -> u128 {
        self.secs as u128 * NANOS_PER_SEC as u128 + self.nanos as u128
    }

    pub fn as_secs_f64(&self) -> f64 {
        self.secs as f64 + self.nanos as f64 / NANOS_PER_SEC as f64
    }

    pub fn as_secs_f32(&self) -> f32 {
        self.as_secs_f64() as f32
    }

    pub fn as_millis_f64(&self) -> f64 {
        self.as_secs_f64() * 1000.0
    }

    pub fn checked_add(self, other: Duration) -> Option<Duration> {
        let mut secs = self.secs.checked_add(other.secs)?;
        let mut nanos = self.nanos + other.nanos;
        if nanos >= NANOS_PER_SEC {
            nanos -= NANOS_PER_SEC;
            secs = secs.checked_add(1)?;
        }
        Some(Duration { secs, nanos })
    }

    pub fn checked_sub(self, other: Duration) -> Option<Duration> {
        let mut secs = self.secs.checked_sub(other.secs)?;
        let nanos = if self.nanos >= other.nanos {
            self.nanos - other.nanos
        } else {
            secs = secs.checked_sub(1)?;
            self.nanos + NANOS_PER_SEC - other.nanos
        };
        Some(Duration { secs, nanos })
    }

    pub fn saturating_sub(self, other: Duration) -> Duration {
        self.checked_sub(other).unwrap_or(Duration::ZERO)
    }

    pub fn saturating_add(self, other: Duration) -> Duration {
        self.checked_add(other).unwrap_or(Duration::MAX)
    }

    pub fn checked_mul(self, factor: u32) -> Option<Duration> {
        let total = self.as_nanos().checked_mul(factor as u128)?;
        let secs = total / NANOS_PER_SEC as u128;
        if secs > u64::MAX as u128 {
            return None;
        }
        Some(Duration { secs: secs as u64, nanos: (total % NANOS_PER_SEC as u128) as u32 })
    }

    pub fn checked_div(self, divisor: u32) -> Option<Duration> {
        if divisor == 0 {
            return None;
        }
        let total = self.as_nanos() / divisor as u128;
        Some(Duration { secs: (total / NANOS_PER_SEC as u128) as u64, nanos: (total % NANOS_PER_SEC as u128) as u32 })
    }

    pub fn mul_f64(self, factor: f64) -> Duration {
        Duration::from_secs_f64(self.as_secs_f64() * factor)
    }

    pub fn div_f64(self, divisor: f64) -> Duration {
        Duration::from_secs_f64(self.as_secs_f64() / divisor)
    }

    pub fn abs_diff(self, other: Duration) -> Duration {
        if self > other { self - other } else { other - self }
    }
}

impl Add for Duration {
    type Output = Duration;

    fn add(self, other: Duration) -> Duration {
        self.checked_add(other).expect("overflow when adding durations")
    }
}

impl AddAssign for Duration {
    fn add_assign(&mut self, other: Duration) {
        *self = *self + other;
    }
}

impl Sub for Duration {
    type Output = Duration;

    fn sub(self, other: Duration) -> Duration {
        self.checked_sub(other).expect("overflow when subtracting durations")
    }
}

impl SubAssign for Duration {
    fn sub_assign(&mut self, other: Duration) {
        *self = *self - other;
    }
}

impl Mul<u32> for Duration {
    type Output = Duration;

    fn mul(self, factor: u32) -> Duration {
        self.checked_mul(factor).expect("overflow when multiplying duration by scalar")
    }
}

impl Div<u32> for Duration {
    type Output = Duration;

    fn div(self, divisor: u32) -> Duration {
        self.checked_div(divisor).expect("divide by zero error when dividing duration by scalar")
    }
}

impl std::iter::Sum for Duration {
    fn sum<I: Iterator<Item = Duration>>(durations: I) -> Duration {
        let mut total = Duration::ZERO;
        for duration in durations {
            total = total + duration;
        }
        total
    }
}

/// Write `integer.fraction` followed by `suffix`, the fraction being
/// `fraction` over `divisor`; as `Duration`'s `{:?}` does: trailing zeros
/// dropped, or exactly `precision` digits, rounded.
fn fmt_decimal(f: &mut fmt::Formatter, integer: u64, fraction: u32, divisor: u32, suffix: &str) -> fmt::Result {
    let mut integer = integer;
    let mut digits = [b'0'; 9];
    let mut count = 0;
    let mut remaining = fraction;
    let mut place = divisor / 10;
    let wanted = f.precision().unwrap_or(9).min(9);
    while place > 0 && count < wanted && (remaining > 0 || f.precision().is_some()) {
        digits[count] = b'0' + (remaining / place) as u8;
        remaining = remaining % place;
        place = place / 10;
        count += 1;
    }
    // Round half up on what was cut off.
    if remaining > 0 && place > 0 && remaining >= place * 5 {
        let mut index = count;
        let mut carry = true;
        while carry && index > 0 {
            index -= 1;
            if digits[index] == b'9' {
                digits[index] = b'0';
            } else {
                digits[index] += 1;
                carry = false;
            }
        }
        if carry {
            integer += 1;
        }
    }
    let precision = f.precision().unwrap_or(count);
    let mut text = integer.to_string();
    if precision > 0 {
        text.push('.');
        let mut index = 0;
        while index < precision {
            text.push(if index < count { digits[index] as char } else { '0' });
            index += 1;
        }
    }
    text.push_str(suffix);
    f.pad(text.as_str())
}

impl fmt::Debug for Duration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.secs > 0 {
            fmt_decimal(f, self.secs, self.nanos, NANOS_PER_SEC, "s")
        } else if self.nanos >= 1_000_000 {
            fmt_decimal(f, (self.nanos / 1_000_000) as u64, self.nanos % 1_000_000, 1_000_000, "ms")
        } else if self.nanos >= 1000 {
            fmt_decimal(f, (self.nanos / 1000) as u64, self.nanos % 1000, 1000, "µs")
        } else {
            fmt_decimal(f, self.nanos as u64, 0, 1, "ns")
        }
    }
}

/// The C library's `struct timespec`.
#[derive(Clone, Copy)]
struct Timespec {
    secs: i64,
    nanos: i64,
}

const CLOCK_REALTIME: i32 = 0;
const CLOCK_MONOTONIC: i32 = 1;

fn now(clock: i32) -> Duration {
    let mut time = Timespec { secs: 0, nanos: 0 };
    unsafe {
        std::libc::clock_gettime(clock, &mut time as *mut Timespec as *mut u8);
    }
    Duration::new(time.secs as u64, time.nanos as u32)
}

/// Sleep for `duration`.
pub(crate) fn sleep_for(duration: Duration) {
    let time = Timespec { secs: duration.secs as i64, nanos: duration.nanos as i64 };
    unsafe {
        std::libc::nanosleep(&time as *const Timespec as *const u8, std::ptr::null_mut());
    }
}

/// A moment, for measuring how long something takes: it only moves forward.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Instant {
    since_boot: Duration,
}

impl Instant {
    pub fn now() -> Instant {
        Instant { since_boot: now(CLOCK_MONOTONIC) }
    }

    pub fn elapsed(&self) -> Duration {
        Instant::now().duration_since(*self)
    }

    /// How much later than `earlier` this is; zero if it is not later.
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        self.since_boot.saturating_sub(earlier.since_boot)
    }

    pub fn saturating_duration_since(&self, earlier: Instant) -> Duration {
        self.duration_since(earlier)
    }

    pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
        self.since_boot.checked_sub(earlier.since_boot)
    }

    pub fn checked_add(&self, duration: Duration) -> Option<Instant> {
        Some(Instant { since_boot: self.since_boot.checked_add(duration)? })
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, duration: Duration) -> Instant {
        Instant { since_boot: self.since_boot + duration }
    }
}

impl Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, duration: Duration) -> Instant {
        Instant { since_boot: self.since_boot - duration }
    }
}

impl Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, earlier: Instant) -> Duration {
        self.duration_since(earlier)
    }
}

/// A moment on the calendar clock, which may be set back and forth.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SystemTime {
    since_epoch: Duration,
}

/// 1970-01-01 00:00:00 UTC.
pub const UNIX_EPOCH: SystemTime = SystemTime { since_epoch: Duration::ZERO };

/// `duration_since` found the given time later than this one.
#[derive(Clone, Debug)]
pub struct SystemTimeError(Duration);

impl SystemTimeError {
    pub fn duration(&self) -> Duration {
        self.0
    }
}

impl fmt::Display for SystemTimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("second time provided was later than self")
    }
}

impl SystemTime {
    pub const UNIX_EPOCH: SystemTime = UNIX_EPOCH;

    pub fn now() -> SystemTime {
        SystemTime { since_epoch: now(CLOCK_REALTIME) }
    }

    pub fn duration_since(&self, earlier: SystemTime) -> Result<Duration, SystemTimeError> {
        match self.since_epoch.checked_sub(earlier.since_epoch) {
            Some(duration) => Ok(duration),
            None => Err(SystemTimeError(earlier.since_epoch - self.since_epoch)),
        }
    }

    pub fn elapsed(&self) -> Result<Duration, SystemTimeError> {
        SystemTime::now().duration_since(*self)
    }
}

impl Add<Duration> for SystemTime {
    type Output = SystemTime;

    fn add(self, duration: Duration) -> SystemTime {
        SystemTime { since_epoch: self.since_epoch + duration }
    }
}

impl Sub<Duration> for SystemTime {
    type Output = SystemTime;

    fn sub(self, duration: Duration) -> SystemTime {
        SystemTime { since_epoch: self.since_epoch - duration }
    }
}
