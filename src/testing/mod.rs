pub mod framework;

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub message: Option<String>,
    pub duration_ms: u128,
}

pub struct TestRunner {
    tests: HashMap<String, fn() -> Result<(), String>>,
    results: Vec<TestResult>,
}

impl TestRunner {
    pub fn new() -> Self {
        TestRunner {
            tests: HashMap::new(),
            results: Vec::new(),
        }
    }

    pub fn register_test(&mut self, name: String, test_fn: fn() -> Result<(), String>) {
        self.tests.insert(name, test_fn);
    }

    pub fn run_all(&mut self) -> TestRunSummary {
        let total = self.tests.len();
        let mut passed = 0;
        let mut failed = 0;

        for (name, test_fn) in &self.tests {
            let start = std::time::Instant::now();
            match test_fn() {
                Ok(_) => {
                    let duration = start.elapsed().as_millis();
                    self.results.push(TestResult {
                        name: name.clone(),
                        passed: true,
                        message: None,
                        duration_ms: duration,
                    });
                    passed += 1;
                }
                Err(e) => {
                    let duration = start.elapsed().as_millis();
                    self.results.push(TestResult {
                        name: name.clone(),
                        passed: false,
                        message: Some(e),
                        duration_ms: duration,
                    });
                    failed += 1;
                }
            }
        }

        TestRunSummary {
            total,
            passed,
            failed,
            results: self.results.clone(),
        }
    }

    pub fn run_single(&mut self, test_name: &str) -> Option<TestResult> {
        if let Some(test_fn) = self.tests.get(test_name) {
            let start = std::time::Instant::now();
            let result = match test_fn() {
                Ok(_) => {
                    let duration = start.elapsed().as_millis();
                    TestResult {
                        name: test_name.to_string(),
                        passed: true,
                        message: None,
                        duration_ms: duration,
                    }
                }
                Err(e) => {
                    let duration = start.elapsed().as_millis();
                    TestResult {
                        name: test_name.to_string(),
                        passed: false,
                        message: Some(e),
                        duration_ms: duration,
                    }
                }
            };
            self.results.push(result.clone());
            Some(result)
        } else {
            None
        }
    }

    pub fn get_results(&self) -> &[TestResult] {
        &self.results
    }
}

#[derive(Debug, Clone)]
pub struct TestRunSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<TestResult>,
}

impl TestRunSummary {
    pub fn print_summary(&self) {
        println!("\n====== Test Results ======");
        println!("Total tests: {}", self.total);
        println!("Passed: {}", self.passed);
        println!("Failed: {}", self.failed);

        if self.failed > 0 {
            println!("\nFailed tests:");
            for result in &self.results {
                if !result.passed {
                    println!(
                        "  ✗ {} ({}ms) - {}",
                        result.name,
                        result.duration_ms,
                        result.message.as_deref().unwrap_or("Unknown error")
                    );
                }
            }
        } else {
            println!("\n✓ All tests passed!");
        }

        let total_time: u128 = self.results.iter().map(|r| r.duration_ms).sum();
        println!("Total time: {}ms\n", total_time);
    }
}

#[macro_export]
macro_rules! assert_eq {
    ($left:expr, $right:expr) => {
        if $left != $right {
            panic!("assertion failed: {:?} != {:?}", $left, $right);
        }
    };
    ($left:expr, $right:expr, $msg:expr) => {
        if $left != $right {
            panic!("assertion failed: {} ({:?} != {:?})", $msg, $left, $right);
        }
    };
}

#[macro_export]
macro_rules! assert {
    ($cond:expr) => {
        if !$cond {
            panic!("assertion failed: condition is false");
        }
    };
    ($cond:expr, $msg:expr) => {
        if !$cond {
            panic!("assertion failed: {}", $msg);
        }
    };
}

#[macro_export]
macro_rules! assert_ne {
    ($left:expr, $right:expr) => {
        if $left == $right {
            panic!("assertion failed: {:?} == {:?}", $left, $right);
        }
    };
    ($left:expr, $right:expr, $msg:expr) => {
        if $left == $right {
            panic!("assertion failed: {} ({:?} == {:?})", $msg, $left, $right);
        }
    };
}

#[macro_export]
macro_rules! test {
    ($name:ident, $body:block) => {
        #[test]
        fn $name() -> Result<(), String> {
            $body
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runner_creation() {
        let runner = TestRunner::new();
        assert_eq!(runner.tests.len(), 0);
    }

    #[test]
    fn test_register_and_run() {
        let mut runner = TestRunner::new();
        runner.register_test("simple_pass".to_string(), || Ok(()));
        runner.register_test("simple_fail".to_string(), || {
            Err("test failed".to_string())
        });

        let summary = runner.run_all();
        assert_eq!(summary.total, 2);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
    }

    #[test]
    fn test_run_single() {
        let mut runner = TestRunner::new();
        runner.register_test("test1".to_string(), || Ok(()));

        let result = runner.run_single("test1");
        assert!(result.is_some());
        let res = result.unwrap();
        assert!(res.passed);
    }
}
