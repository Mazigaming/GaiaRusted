//! Extended Math Functions for GaiaRusted Standard Library
//!
//! Provides mathematical functions:
//! - Trigonometric (sin, cos, tan, asin, acos, atan)
//! - Hyperbolic (sinh, cosh, tanh)
//! - Exponential (exp, log, log10, log2)
//! - Power & roots (sqrt, cbrt, pow)
//! - Rounding (floor, ceil, round, trunc)
//! - Absolute value and min/max

use std::f64::consts;

/// Mathematical constants
pub struct Math;

impl Math {
    /// Pi
    pub const PI: f64 = consts::PI;
    
    /// Euler's number (e)
    pub const E: f64 = consts::E;
    
    /// Tau (2π)
    pub const TAU: f64 = 2.0 * consts::PI;
    
    /// Square root of 2
    pub const SQRT_2: f64 = consts::SQRT_2;
    
    /// Inverse square root of 2
    pub const FRAC_1_SQRT_2: f64 = consts::FRAC_1_SQRT_2;
}

/// Trigonometric functions
pub trait Trigonometric {
    /// Sine (radians)
    fn sin(self) -> f64;
    
    /// Cosine (radians)
    fn cos(self) -> f64;
    
    /// Tangent (radians)
    fn tan(self) -> f64;
    
    /// Arc sine
    fn asin(self) -> f64;
    
    /// Arc cosine
    fn acos(self) -> f64;
    
    /// Arc tangent
    fn atan(self) -> f64;
}

/// Hyperbolic functions
pub trait Hyperbolic {
    /// Hyperbolic sine
    fn sinh(self) -> f64;
    
    /// Hyperbolic cosine
    fn cosh(self) -> f64;
    
    /// Hyperbolic tangent
    fn tanh(self) -> f64;
}

/// Exponential and logarithmic functions
pub trait Exponential {
    /// e raised to the power of self
    fn exp(self) -> f64;
    
    /// Natural logarithm (base e)
    fn ln(self) -> f64;
    
    /// Logarithm base 10
    fn log10(self) -> f64;
    
    /// Logarithm base 2
    fn log2(self) -> f64;
}

/// Power and root functions
pub trait PowerRoot {
    /// Square root
    fn sqrt(self) -> f64;
    
    /// Cube root
    fn cbrt(self) -> f64;
    
    /// Raise to power
    fn pow(self, exp: f64) -> f64;
    
    /// Reciprocal (1/x)
    fn recip(self) -> f64;
}

/// Rounding functions
pub trait Rounding {
    /// Round down to nearest integer
    fn floor(self) -> f64;
    
    /// Round up to nearest integer
    fn ceil(self) -> f64;
    
    /// Round to nearest integer
    fn round(self) -> f64;
    
    /// Truncate toward zero
    fn trunc(self) -> f64;
    
    /// Get absolute value
    fn abs(self) -> f64;
    
    /// Get fractional part
    fn fract(self) -> f64;
}

/// Implement trigonometric functions for f64
impl Trigonometric for f64 {
    fn sin(self) -> f64 {
        // Use builtin Rust sin
        f64::sin(self)
    }
    
    fn cos(self) -> f64 {
        f64::cos(self)
    }
    
    fn tan(self) -> f64 {
        f64::tan(self)
    }
    
    fn asin(self) -> f64 {
        f64::asin(self)
    }
    
    fn acos(self) -> f64 {
        f64::acos(self)
    }
    
    fn atan(self) -> f64 {
        f64::atan(self)
    }
}

/// Implement hyperbolic functions for f64
impl Hyperbolic for f64 {
    fn sinh(self) -> f64 {
        f64::sinh(self)
    }
    
    fn cosh(self) -> f64 {
        f64::cosh(self)
    }
    
    fn tanh(self) -> f64 {
        f64::tanh(self)
    }
}

/// Implement exponential functions for f64
impl Exponential for f64 {
    fn exp(self) -> f64 {
        f64::exp(self)
    }
    
    fn ln(self) -> f64 {
        f64::ln(self)
    }
    
    fn log10(self) -> f64 {
        f64::log10(self)
    }
    
    fn log2(self) -> f64 {
        f64::log2(self)
    }
}

/// Implement power/root functions for f64
impl PowerRoot for f64 {
    fn sqrt(self) -> f64 {
        f64::sqrt(self)
    }
    
    fn cbrt(self) -> f64 {
        f64::cbrt(self)
    }
    
    fn pow(self, exp: f64) -> f64 {
        f64::powf(self, exp)
    }
    
    fn recip(self) -> f64 {
        1.0 / self
    }
}

/// Implement rounding functions for f64
impl Rounding for f64 {
    fn floor(self) -> f64 {
        f64::floor(self)
    }
    
    fn ceil(self) -> f64 {
        f64::ceil(self)
    }
    
    fn round(self) -> f64 {
        f64::round(self)
    }
    
    fn trunc(self) -> f64 {
        f64::trunc(self)
    }
    
    fn abs(self) -> f64 {
        f64::abs(self)
    }
    
    fn fract(self) -> f64 {
        f64::fract(self)
    }
}

/// Min/Max operations
pub fn min<T: PartialOrd>(a: T, b: T) -> T {
    if a < b { a } else { b }
}

pub fn max<T: PartialOrd>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

/// Clamp value between min and max
pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Two-argument arctangent
pub fn atan2(y: f64, x: f64) -> f64 {
    f64::atan2(y, x)
}

/// Hypotenuse (sqrt(x² + y²))
pub fn hypot(x: f64, y: f64) -> f64 {
    f64::hypot(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert!((Math::PI - 3.14159265).abs() < 0.0001);
        assert!((Math::E - 2.71828182).abs() < 0.0001);
    }

    #[test]
    fn test_trigonometric() {
        let pi = Math::PI;
        assert!((0.0_f64.sin() - 0.0).abs() < 1e-10);
        assert!((0.0_f64.cos() - 1.0).abs() < 1e-10);
        assert!(((pi / 2.0).sin() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_exponential() {
        assert!(((1.0_f64.exp() - Math::E).abs()) < 0.0001);
        assert!((Math::E.ln() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_power_root() {
        assert!((4.0_f64.sqrt() - 2.0).abs() < 1e-10);
        assert!((27.0_f64.cbrt() - 3.0).abs() < 1e-10);
        assert!((2.0_f64.pow(3.0) - 8.0).abs() < 1e-10);
    }

    #[test]
    fn test_rounding() {
        assert_eq!(3.7_f64.floor(), 3.0);
        assert_eq!(3.2_f64.ceil(), 4.0);
        assert_eq!(3.5_f64.round(), 4.0);
        assert_eq!(3.9_f64.trunc(), 3.0);
    }

    #[test]
    fn test_min_max() {
        assert_eq!(min(5, 3), 3);
        assert_eq!(max(5, 3), 5);
    }

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(5, 1, 3), 3);
        assert_eq!(clamp(2, 1, 3), 2);
        assert_eq!(clamp(0, 1, 3), 1);
    }

    #[test]
    fn test_hypot() {
        let result = hypot(3.0, 4.0);
        assert!((result - 5.0).abs() < 1e-10);
    }
}
