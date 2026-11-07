//! Performance regression testing.
//!
//! CI guards and performance budgets to ensure the simulator stays smooth
//! across changes.


/// A performance budget for a metric.
#[derive(Clone, Debug, PartialEq)]
pub struct PerformanceBudget {
    pub name: &'static str,
    pub max_value: f64,
    pub unit: &'static str,
    pub critical: bool, // if false, only warn
}

/// Performance regression report.
#[derive(Clone, Debug)]
pub struct PerformanceReport {
    pub passed: bool,
    pub failures: Vec<(String, f64, f64)>, // (name, actual, max)
    pub warnings: Vec<(String, f64, f64)>,
}

/// Performance regression checker.
#[derive(Clone, Debug, Default)]
pub struct PerformanceChecker {
    pub budgets: Vec<PerformanceBudget>,
}

impl PerformanceChecker {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_budget(&mut self, budget: PerformanceBudget) {
        self.budgets.push(budget);
    }
    /// Check a measured value against budgets.
    pub fn check(&self, measurements: &[(&str, f64)]) -> PerformanceReport {
        let mut failures = Vec::new();
        let mut warnings = Vec::new();
        for (name, value) in measurements {
            if let Some(budget) = self.budgets.iter().find(|b| b.name == *name) {
                if *value > budget.max_value {
                    if budget.critical {
                        failures.push((name.to_string(), *value, budget.max_value));
                    } else {
                        warnings.push((name.to_string(), *value, budget.max_value));
                    }
                }
            }
        }
        PerformanceReport {
            passed: failures.is_empty(),
            failures,
            warnings,
        }
    }
    /// Check frame time budget.
    pub fn check_frame_time(&self, frame_ms: f64) -> PerformanceReport {
        self.check(&[("frame_time_ms", frame_ms)])
    }
    pub fn check_draw_calls(&self, draw_calls: u32) -> PerformanceReport {
        self.check(&[("draw_calls", draw_calls as f64)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn checker_passes_when_under_budget() {
        let mut checker = PerformanceChecker::new();
        checker.add_budget(PerformanceBudget {
            name: "frame_time_ms",
            max_value: 16.67,
            unit: "ms",
            critical: true,
        });
        let report = checker.check_frame_time(8.0);
        assert!(report.passed);
        assert!(report.failures.is_empty());
    }
    #[test] fn checker_fails_when_over_budget() {
        let mut checker = PerformanceChecker::new();
        checker.add_budget(PerformanceBudget {
            name: "frame_time_ms",
            max_value: 16.67,
            unit: "ms",
            critical: true,
        });
        let report = checker.check_frame_time(30.0);
        assert!(!report.passed);
        assert_eq!(report.failures.len(), 1);
    }
    #[test] fn non_critical_only_warns() {
        let mut checker = PerformanceChecker::new();
        checker.add_budget(PerformanceBudget {
            name: "draw_calls",
            max_value: 5000.0,
            unit: "count",
            critical: false,
        });
        let report = checker.check_draw_calls(8000);
        assert!(report.passed); // non-critical, so still passes
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.failures.len(), 0);
    }
    #[test] fn multiple_budgets() {
        let mut checker = PerformanceChecker::new();
        checker.add_budget(PerformanceBudget {
            name: "frame_time_ms",
            max_value: 16.67,
            unit: "ms",
            critical: true,
        });
        checker.add_budget(PerformanceBudget {
            name: "draw_calls",
            max_value: 5000.0,
            unit: "count",
            critical: true,
        });
        let report = checker.check(&[("frame_time_ms", 8.0), ("draw_calls", 6000.0)]);
        assert!(!report.passed);
        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.failures[0].0, "draw_calls");
    }
}
