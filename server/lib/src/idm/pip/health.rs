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
        assert!(state.last_check.is_none());
        assert!(state.last_error.is_none());
    }

    #[test]
    fn test_default_state() {
        let state = PipHealthState::default();
        assert_eq!(state.status, PipHealthStatus::Unknown);
    }

    #[test]
    fn test_success_transitions_to_healthy() {
        let config = test_config();
        let mut state = PipHealthState::new();

        state.record_success(&config);
        assert_eq!(state.status, PipHealthStatus::Unknown);
        assert_eq!(state.consecutive_successes, 1);
        assert_eq!(state.consecutive_failures, 0);

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
        assert_eq!(state.consecutive_failures, 1);
        assert!(state.last_error.is_some());

        state.record_failure(&config, "error2".to_string());
        assert_eq!(state.status, PipHealthStatus::Degraded);
        assert_eq!(state.consecutive_failures, 2);

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
        assert!(state.last_error.is_none());
    }

    #[test]
    fn test_failure_clears_successes() {
        let config = test_config();
        let mut state = PipHealthState::new();

        state.record_success(&config);
        state.record_success(&config);
        assert_eq!(state.consecutive_successes, 2);

        state.record_failure(&config, "error".to_string());
        assert_eq!(state.consecutive_successes, 0);
        assert_eq!(state.consecutive_failures, 1);
    }

    #[test]
    fn test_last_check_updated_on_success() {
        let config = test_config();
        let mut state = PipHealthState::new();

        assert!(state.last_check.is_none());
        state.record_success(&config);
        assert!(state.last_check.is_some());
    }

    #[test]
    fn test_last_check_updated_on_failure() {
        let config = test_config();
        let mut state = PipHealthState::new();

        assert!(state.last_check.is_none());
        state.record_failure(&config, "error".to_string());
        assert!(state.last_check.is_some());
    }

    #[test]
    fn test_last_error_set_on_failure() {
        let config = test_config();
        let mut state = PipHealthState::new();

        state.record_failure(&config, "connection timeout".to_string());
        assert_eq!(state.last_error, Some("connection timeout".to_string()));

        state.record_failure(&config, "server error".to_string());
        assert_eq!(state.last_error, Some("server error".to_string()));
    }

    #[test]
    fn test_last_error_cleared_on_success() {
        let config = test_config();
        let mut state = PipHealthState::new();

        state.record_failure(&config, "error".to_string());
        assert!(state.last_error.is_some());

        state.record_success(&config);
        assert!(state.last_error.is_none());
    }

    #[test]
    fn test_multiple_failures_before_threshold() {
        let config = PipHealthCheckConfig {
            interval_secs: 60,
            failure_threshold: 5,
            success_threshold: 2,
            timeout_secs: 5,
        };
        let mut state = PipHealthState::new();

        for i in 1..=4 {
            state.record_failure(&config, format!("error{}", i));
            assert_eq!(state.status, PipHealthStatus::Degraded);
        }

        state.record_failure(&config, "error5".to_string());
        assert_eq!(state.status, PipHealthStatus::Unhealthy);
    }

    #[test]
    fn test_multiple_successes_before_threshold() {
        let config = PipHealthCheckConfig {
            interval_secs: 60,
            failure_threshold: 3,
            success_threshold: 5,
            timeout_secs: 5,
        };
        let mut state = PipHealthState::new();

        state.record_failure(&config, "error".to_string());
        assert_eq!(state.status, PipHealthStatus::Degraded);

        for _ in 1..=4 {
            state.record_success(&config);
            assert_eq!(state.status, PipHealthStatus::Degraded);
        }

        state.record_success(&config);
        assert_eq!(state.status, PipHealthStatus::Healthy);
    }

    #[test]
    fn test_recovery_from_unhealthy() {
        let config = test_config();
        let mut state = PipHealthState::new();

        state.record_failure(&config, "error1".to_string());
        state.record_failure(&config, "error2".to_string());
        state.record_failure(&config, "error3".to_string());
        assert_eq!(state.status, PipHealthStatus::Unhealthy);

        state.record_success(&config);
        assert_eq!(state.status, PipHealthStatus::Unhealthy);

        state.record_success(&config);
        assert_eq!(state.status, PipHealthStatus::Healthy);
    }

    #[test]
    fn test_failure_threshold_one() {
        let config = PipHealthCheckConfig {
            interval_secs: 60,
            failure_threshold: 1,
            success_threshold: 1,
            timeout_secs: 5,
        };
        let mut state = PipHealthState::new();

        state.record_failure(&config, "error".to_string());
        assert_eq!(state.status, PipHealthStatus::Unhealthy);
    }

    #[test]
    fn test_success_threshold_one() {
        let config = PipHealthCheckConfig {
            interval_secs: 60,
            failure_threshold: 3,
            success_threshold: 1,
            timeout_secs: 5,
        };
        let mut state = PipHealthState::new();

        state.record_success(&config);
        assert_eq!(state.status, PipHealthStatus::Healthy);
    }

    #[test]
    fn test_alternating_success_failure() {
        let config = test_config();
        let mut state = PipHealthState::new();

        state.record_success(&config);
        state.record_success(&config);
        assert_eq!(state.status, PipHealthStatus::Healthy);

        state.record_failure(&config, "error".to_string());
        assert_eq!(state.status, PipHealthStatus::Degraded);

        state.record_success(&config);
        state.record_success(&config);
        assert_eq!(state.status, PipHealthStatus::Healthy);
    }

    #[test]
    fn test_cloned_state_preserves_values() {
        let config = test_config();
        let mut state = PipHealthState::new();
        state.record_success(&config);
        state.record_failure(&config, "error".to_string());

        let cloned = state.clone();
        assert_eq!(cloned.status, state.status);
        assert_eq!(cloned.consecutive_failures, state.consecutive_failures);
        assert_eq!(cloned.consecutive_successes, state.consecutive_successes);
        assert_eq!(cloned.last_error, state.last_error);
    }

    #[test]
    fn test_health_status_can_retrieve() {
        assert!(PipHealthStatus::Healthy.can_retrieve());
        assert!(PipHealthStatus::Degraded.can_retrieve());
        assert!(!PipHealthStatus::Unhealthy.can_retrieve());
        assert!(!PipHealthStatus::Unknown.can_retrieve());
    }

    #[test]
    fn test_health_status_is_healthy() {
        assert!(PipHealthStatus::Healthy.is_healthy());
        assert!(!PipHealthStatus::Degraded.is_healthy());
        assert!(!PipHealthStatus::Unhealthy.is_healthy());
        assert!(!PipHealthStatus::Unknown.is_healthy());
    }
}
