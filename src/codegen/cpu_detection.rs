//! CPU Capability Detection for SIMD Code Generation
//!
//! Detects available CPU features:
//! - SSE2 (baseline for x86-64)
//! - SSE4.1, SSE4.2
//! - AVX (256-bit vectors)
//! - AVX2 (with FMA)
//! - AVX-512 (future)

use std::process::Command;

/// CPU feature set
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CPUFeatures {
    pub sse2: bool,
    pub sse41: bool,
    pub sse42: bool,
    pub avx: bool,
    pub avx2: bool,
    pub fma: bool,
    pub bmi1: bool,
    pub bmi2: bool,
}

impl CPUFeatures {
    /// Detect CPU capabilities from /proc/cpuinfo
    pub fn detect() -> Self {
        Self::detect_from_cpuinfo()
            .or_else(|_| Self::detect_from_cpuid())
            .unwrap_or_else(|_| Self::default_features())
    }

    /// Get default features (conservative, x86-64 baseline)
    fn default_features() -> Self {
        CPUFeatures {
            sse2: true,    // x86-64 baseline
            sse41: true,   // Common on modern CPUs
            sse42: true,
            avx: false,    // Conservative default
            avx2: false,
            fma: false,
            bmi1: false,
            bmi2: false,
        }
    }

    /// Detect from /proc/cpuinfo (Linux)
    fn detect_from_cpuinfo() -> Result<Self, String> {
        let output = std::fs::read_to_string("/proc/cpuinfo")
            .map_err(|e| format!("Cannot read /proc/cpuinfo: {}", e))?;

        let mut features = Self::default_features();

        for line in output.lines() {
            if line.starts_with("flags") {
                let flags = line.split(':').nth(1).unwrap_or("");
                features.sse41 = flags.contains("sse4_1");
                features.sse42 = flags.contains("sse4_2");
                features.avx = flags.contains("avx");
                features.avx2 = flags.contains("avx2");
                features.fma = flags.contains("fma");
                features.bmi1 = flags.contains("bmi1");
                features.bmi2 = flags.contains("bmi2");
                break;
            }
        }

        Ok(features)
    }

    /// Detect from CPUID instruction (if available)
    fn detect_from_cpuid() -> Result<Self, String> {
        // Try to run a hypothetical cpuid utility
        let output = Command::new("lscpu")
            .output()
            .map_err(|_| "lscpu not available")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut features = Self::default_features();

        for line in stdout.lines() {
            if line.contains("Flags") || line.contains("flags") {
                features.sse41 = line.contains("sse4_1");
                features.sse42 = line.contains("sse4_2");
                features.avx = line.contains("avx");
                features.avx2 = line.contains("avx2");
                features.fma = line.contains("fma");
                features.bmi1 = line.contains("bmi1");
                features.bmi2 = line.contains("bmi2");
                break;
            }
        }

        Ok(features)
    }

    /// Get best available SIMD level
    pub fn best_simd_level(&self) -> SIMDLevel {
        if self.avx2 {
            SIMDLevel::AVX2
        } else if self.avx {
            SIMDLevel::AVX
        } else if self.sse42 {
            SIMDLevel::SSE42
        } else {
            SIMDLevel::SSE2  // Baseline
        }
    }

    /// Check if AVX2 is available
    pub fn has_avx2(&self) -> bool {
        self.avx2
    }

    /// Check if AVX is available
    pub fn has_avx(&self) -> bool {
        self.avx
    }

    /// Check if SSE4.2 is available
    pub fn has_sse42(&self) -> bool {
        self.sse42
    }

    /// Check if FMA3 is available (usually with AVX)
    pub fn has_fma(&self) -> bool {
        self.fma
    }
}

/// SIMD capability levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SIMDLevel {
    /// 128-bit vectors (SSE2/SSE4.x)
    SSE2 = 0,
    /// 128-bit with SSE4.2
    SSE42 = 1,
    /// 256-bit vectors
    AVX = 2,
    /// 256-bit with FMA and other improvements
    AVX2 = 3,
}

impl SIMDLevel {
    /// Vector width in bytes
    pub fn vector_width(&self) -> usize {
        match self {
            SIMDLevel::SSE2 | SIMDLevel::SSE42 => 16,
            SIMDLevel::AVX | SIMDLevel::AVX2 => 32,
        }
    }

    /// Number of i64 elements per vector
    pub fn i64_lanes(&self) -> usize {
        self.vector_width() / 8
    }

    /// Number of i32 elements per vector
    pub fn i32_lanes(&self) -> usize {
        self.vector_width() / 4
    }

    /// Number of f64 elements per vector
    pub fn f64_lanes(&self) -> usize {
        self.vector_width() / 8
    }

    /// Number of f32 elements per vector
    pub fn f32_lanes(&self) -> usize {
        self.vector_width() / 4
    }

    /// String representation for code generation
    pub fn as_str(&self) -> &'static str {
        match self {
            SIMDLevel::SSE2 => "sse2",
            SIMDLevel::SSE42 => "sse42",
            SIMDLevel::AVX => "avx",
            SIMDLevel::AVX2 => "avx2",
        }
    }
}

impl std::fmt::Display for SIMDLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_cpu_features() {
        let features = CPUFeatures::default_features();
        assert!(features.sse2);  // x86-64 baseline
    }

    #[test]
    fn test_simd_level_widths() {
        assert_eq!(SIMDLevel::SSE2.vector_width(), 16);
        assert_eq!(SIMDLevel::AVX.vector_width(), 32);
        assert_eq!(SIMDLevel::AVX2.vector_width(), 32);
    }

    #[test]
    fn test_simd_lane_counts() {
        assert_eq!(SIMDLevel::SSE2.i64_lanes(), 2);
        assert_eq!(SIMDLevel::SSE2.i32_lanes(), 4);
        assert_eq!(SIMDLevel::AVX2.i64_lanes(), 4);
        assert_eq!(SIMDLevel::AVX2.i32_lanes(), 8);
    }

    #[test]
    fn test_simd_level_ordering() {
        assert!(SIMDLevel::SSE2 < SIMDLevel::AVX);
        assert!(SIMDLevel::AVX < SIMDLevel::AVX2);
        assert!(SIMDLevel::SSE42 < SIMDLevel::AVX2);
    }

    #[test]
    fn test_cpu_detection() {
        // This will use actual CPU detection on the system
        let features = CPUFeatures::detect();
        // At minimum, all x86-64 CPUs have SSE2
        assert!(features.sse2);
    }

    #[test]
    fn test_best_simd_level_fallback() {
        let features = CPUFeatures::default_features();
        // Default includes SSE4.2 which is common on modern CPUs
        assert!(features.best_simd_level() >= SIMDLevel::SSE2);
    }
}
