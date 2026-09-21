// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;
use rand::Rng;

use limits::{
    RETRY_INITIAL_DELAY_MS, RETRY_JITTER_FACTOR, RETRY_MAX_ATTEMPTS, RETRY_MAX_DELAY_MS,
};

/// Truncated exponential backoff with jitter for reconnecting to peers.
#[derive(Debug, Clone)]
pub struct ReconnectPolicy {
    current_attempt: u32,
    max_attempts: u32,
    initial_delay: Duration,
    max_delay: Duration,
    jitter_factor: f64,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl ReconnectPolicy {
    pub fn new() -> Self {
        Self {
            current_attempt: 0,
            max_attempts: RETRY_MAX_ATTEMPTS,
            initial_delay: Duration::from_millis(RETRY_INITIAL_DELAY_MS),
            max_delay: Duration::from_millis(RETRY_MAX_DELAY_MS),
            jitter_factor: RETRY_JITTER_FACTOR,
        }
    }

    /// Reset attempt counter on successful connection.
    pub fn reset(&mut self) {
        self.current_attempt = 0;
    }

    /// Current attempt number.
    pub fn attempt(&self) -> u32 {
        self.current_attempt
    }

    /// Compute next backoff delay, or `None` if `max_attempts` has been reached.
    pub fn next_delay(&mut self) -> Option<Duration> {
        if self.current_attempt >= self.max_attempts {
            return None;
        }

        let base_multiplier = 2u64.saturating_pow(self.current_attempt);
        let base_delay_ms = (self.initial_delay.as_millis() as u64)
            .saturating_mul(base_multiplier)
            .min(self.max_delay.as_millis() as u64);

        self.current_attempt += 1;

        // Apply Â±25% random jitter
        let mut rng = rand::thread_rng();
        let jitter_range = (base_delay_ms as f64) * self.jitter_factor;
        let jitter = rng.gen_range(-jitter_range..=jitter_range);
        let final_delay_ms = ((base_delay_ms as f64) + jitter).max(0.0) as u64;

        Some(Duration::from_millis(final_delay_ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_caps_and_terminates() {
        let mut policy = ReconnectPolicy::new();
        let mut count = 0;
        while let Some(delay) = policy.next_delay() {
            count += 1;
            let max_allowed = (RETRY_MAX_DELAY_MS as f64 * (1.0 + RETRY_JITTER_FACTOR)) as u64;
            assert!(delay.as_millis() as u64 <= max_allowed + 100);
        }
        assert_eq!(count, RETRY_MAX_ATTEMPTS);
        assert_eq!(policy.next_delay(), None);

        policy.reset();
        assert_eq!(policy.attempt(), 0);
        assert!(policy.next_delay().is_some());
    }
}
