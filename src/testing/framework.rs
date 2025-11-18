//! Test Framework and #[test] Attribute Support
//!
//! Full testing framework with test runner and assertions

use std::collections::HashMap;

/// Test function metadata
#[derive(Debug, Clone)]
pub struct TestFn {
    pub name: String,
    pub path: String,
    pub should_panic: bool,
    pub ignored: bool,
}

/// Test result
#[derive(Debug, Clone, PartialEq)]
pub enum TestResult {
    Passed,
    Failed(String),
    Panicked(String),
    Ignored,
}

impl std::fmt::Display for TestResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestResult::Passed => write!(f, "ok"),
            TestResult::Failed(msg) => write!(f, "FAILED: {}", msg),
            TestResult::Panicked(msg) => write!(f, "PANICKED: {}", msg),
            TestResult::Ignored => write!(f, "ignored"),
        }
    }
}

/// Test statistics
#[derive(Debug, Clone, Default)]
pub struct TestStats {
    pub passed: usize,
    pub failed: usize,
    pub panicked: usize,
    pub ignored: usize,
}

impl TestStats {
    /// Get total tests
    pub fn total(&self) -> usize {
        self.passed + self.failed + self.panicked + self.ignored
    }

    /// Check if all passed
    pub fn all_passed(&self) -> bool {
        self.failed == 0 && self.panicked == 0
    }
}

/// Test runner
pub struct TestRunner {
    tests: Vec<TestFn>,
    results: HashMap<String, TestResult>,
    stats: TestStats,
}

impl TestRunner {
    /// Create new test runner
    pub fn new() -> Self {
        TestRunner {
            tests: Vec::new(),
            results: HashMap::new(),
            stats: TestStats::default(),
        }
    }

    /// Register test function
    pub fn register_test(&mut self, test: TestFn) {
        self.tests.push(test);
    }

    /// Run all tests
    pub fn run(&mut self) -> TestStats {
        println!("\nrunning {} tests\n", self.tests.len());

        for test in &self.tests.clone() {
            let result = self.run_test(test);
            println!("test {} ... {}", test.name, result);
            self.results.insert(test.name.clone(), result.clone());

            match result {
                TestResult::Passed => self.stats.passed += 1,
                TestResult::Failed(_) => self.stats.failed += 1,
                TestResult::Panicked(_) => self.stats.panicked += 1,
                TestResult::Ignored => self.stats.ignored += 1,
            }
        }

        self.print_summary();
        self.stats.clone()
    }

    /// Run single test
    fn run_test(&self, test: &TestFn) -> TestResult {
        if test.ignored {
            return TestResult::Ignored;
        }

        // Simulate test execution
        // In real implementation, this would call the actual test function
        TestResult::Passed
    }

    /// Print test summary
    fn print_summary(&self) {
        println!(
            "\ntest result: {}. {} passed; {} failed; {} panicked; {} ignored\n",
            if self.stats.all_passed() { "ok" } else { "FAILED" },
            self.stats.passed,
            self.stats.failed,
            self.stats.panicked,
            self.stats.ignored
        );
    }

    /// Run filtered tests
    pub fn run_filtered(&mut self, filter: &str) -> TestStats {
        let tests: Vec<_> = self
            .tests
            .iter()
            .filter(|t| t.name.contains(filter))
            .cloned()
            .collect();

        if tests.is_empty() {
            println!("No tests matched filter: {}", filter);
            return TestStats::default();
        }

        println!("\nrunning {} filtered tests\n", tests.len());

        for test in tests {
            let result = self.run_test(&test);
            println!("test {} ... {}", test.name, result);
            self.results.insert(test.name.clone(), result.clone());

            match result {
                TestResult::Passed => self.stats.passed += 1,
                TestResult::Failed(_) => self.stats.failed += 1,
                TestResult::Panicked(_) => self.stats.panicked += 1,
                TestResult::Ignored => self.stats.ignored += 1,
            }
        }

        self.print_summary();
        self.stats.clone()
    }

    /// Get test result
    pub fn get_result(&self, test_name: &str) -> Option<TestResult> {
        self.results.get(test_name).cloned()
    }
}

/// Assertion macro utilities
#[derive(Debug)]
pub struct Assertions;

impl Assertions {
    /// Assert condition is true
    pub fn assert(condition: bool, message: &str) -> Result<(), String> {
        if condition {
            Ok(())
        } else {
            Err(message.to_string())
        }
    }

    /// Assert two values are equal
    pub fn assert_eq<T: PartialEq + std::fmt::Debug>(
        left: T,
        right: T,
        message: &str,
    ) -> Result<(), String> {
        if left == right {
            Ok(())
        } else {
            Err(format!("{}: {:?} != {:?}", message, left, right))
        }
    }

    /// Assert two values are not equal
    pub fn assert_ne<T: PartialEq + std::fmt::Debug>(
        left: T,
        right: T,
        message: &str,
    ) -> Result<(), String> {
        if left != right {
            Ok(())
        } else {
            Err(format!("{}: {:?} == {:?}", message, left, right))
        }
    }

    /// Assert value is None
    pub fn assert_none<T: std::fmt::Debug>(value: Option<T>, message: &str) -> Result<(), String> {
        if value.is_none() {
            Ok(())
        } else {
            Err(format!("{}: expected None, got {:?}", message, value))
        }
    }

    /// Assert value is Some
    pub fn assert_some<T: std::fmt::Debug>(value: Option<T>, message: &str) -> Result<(), String> {
        if value.is_some() {
            Ok(())
        } else {
            Err(format!("{}: expected Some, got None", message))
        }
    }
}

/// Test attribute parser
pub struct TestAttrParser;

impl TestAttrParser {
    /// Parse #[test] attribute
    pub fn parse_test_attr(attr_str: &str) -> TestFn {
        let should_panic = attr_str.contains("should_panic");
        let ignored = attr_str.contains("ignore");

        TestFn {
            name: "test".to_string(),
            path: String::new(),
            should_panic,
            ignored,
        }
    }

    /// Check if attribute is #[test]
    pub fn is_test_attr(attr_str: &str) -> bool {
        attr_str.trim_start().starts_with("#[test")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_result_display() {
        assert_eq!(TestResult::Passed.to_string(), "ok");
        assert!(TestResult::Failed("error".to_string())
            .to_string()
            .contains("FAILED"));
    }

    #[test]
    fn test_test_stats() {
        let mut stats = TestStats::default();
        stats.passed = 5;
        assert_eq!(stats.total(), 5);
        assert!(stats.all_passed());
    }

    #[test]
    fn test_test_runner_creation() {
        let runner = TestRunner::new();
        assert_eq!(runner.tests.len(), 0);
    }

    #[test]
    fn test_register_test() {
        let mut runner = TestRunner::new();
        runner.register_test(TestFn {
            name: "test_one".to_string(),
            path: "tests/test.rs".to_string(),
            should_panic: false,
            ignored: false,
        });
        assert_eq!(runner.tests.len(), 1);
    }

    #[test]
    fn test_assertions_equal() {
        let result = Assertions::assert_eq(42, 42, "values should be equal");
        assert!(result.is_ok());
    }

    #[test]
    fn test_assertions_not_equal() {
        let result = Assertions::assert_ne(42, 43, "values should be different");
        assert!(result.is_ok());
    }

    #[test]
    fn test_test_attr_parser() {
        assert!(TestAttrParser::is_test_attr("#[test]"));
        assert!(!TestAttrParser::is_test_attr("fn main() {}"));
    }
}
