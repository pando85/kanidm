//! PIP Health State Tracking
//!
//! Shared health state tracking logic for all PIP implementations.

use std::time::Instant;

use super::config::PipHealthCheckConfig;
use super::PipHealthStatus;

/// Internal health state tracking shared by all PIP implementations
#[derive(Debug, Clone)]
pub struct PipHealthState {
    pub status: PipHealthStatus,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub last_check: Option<Instant>,
    pub last_error: Option<String>,
}

impl PipHealthState {
    pub fn new() -> Self {
        PipHealthState {
            status: PipHealthStatus::Unknown,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_check: None,
            last_error: None,
        }
    }

    pub fn record_success(&mut self, config: &PipHealthCheckConfig) {
        self.consecutive_failures = 0;
        self.consecutive_successes += 1;

        if self.consecutive_successes >= config.success_threshold {
            self.status = PipHealthStatus::Healthy;
        }

        self.last_check = Some(Instant::now());
        self.last_error = None;
    }

    pub fn record_failure(&mut self, config: &PipHealthCheckConfig, error: String) {
        self.consecutive_successes = 0;
        self.consecutive_failures += 1;

        if self.consecutive_failures >= config.failure_threshold {
            self.status = PipHealthStatus::Unhealthy;
        } else if self.consecutive_failures > 0 {
            self.status = PipHealthStatus::Degraded;
        }

        self.last_check = Some(Instant::now());
        self.last_error = Some(error);
    }
}

impl Default for PipHealthState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> PipHealthCheckConfig {
        PipHealthCheckConfig {
            interval_secs: 60,
            failure_threshold: 3,
            success_threshold: 2,
            timeout_secs: 5,
        }
    }

    #[test]
    fn test_initial_state() {
        let state = PipHealthState::new();
        assert_eq!(state.status, PipHealthStatus::Unknown);
        assert_eq!(state.consecutive_failures, 0);
        assert_eq!(state.consecutive_successes, 0);
    }

    #[test]
    fn test_success_transitions_to_healthy() {
        let config = test_config();
        let mut state = PipHealthState::new();

        state.record_success(&config);
        assert_eq!(state.status, PipHealthStatus::Unknown);

        state.record_success(&config);
        assert_eq!(state.status, PipHealthStatus::Healthy);
        assert_eq!(state.consecutive_failures, 0);
    }

    #[test]
    fn test_failure_transitions_to_unhealthy() {
        let config = test_config();
        let mut state = PipHealthState::new();

        state.record_failure(&config, "error1".to_string());
        assert_eq!(state.status, PipHealthStatus::Degraded);

        state.record_failure(&config, "error2".to_string());
        assert_eq!(state.status, PipHealthStatus::Degraded);

        state.record_failure(&config, "error3".to_string());
        assert_eq!(state.status, PipHealthStatus::Unhealthy);
        assert_eq!(state.consecutive_successes, 0);
    }

    #[test]
    fn test_success_clears_failures() {
        let config = test_config();
        let mut state = PipHealthState::new();

        state.record_failure(&config, "error".to_string());
        assert_eq!(state.consecutive_failures, 1);

        state.record_success(&config);
        assert_eq!(state.consecutive_failures, 0);
        assert_eq!(state.consecutive_successes, 1);
    }
}
