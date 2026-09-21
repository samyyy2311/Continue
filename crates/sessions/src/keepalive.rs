// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::time::{Duration, Instant};

use limits::{KEEPALIVE_INTERVAL_SECS, KEEPALIVE_TIMEOUT_SECS};
use protocol::v1::{session_envelope::Body, Ping, Pong, SessionEnvelope};

/// Tracks Ping/Pong sequences and keepalive health.
pub struct KeepaliveTracker {
    last_received: Instant,
    last_sent_ping: Option<Instant>,
    current_seq: u64,
    interval: Duration,
    timeout: Duration,
}

impl Default for KeepaliveTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl KeepaliveTracker {
    pub fn new() -> Self {
        Self {
            last_received: Instant::now(),
            last_sent_ping: None,
            current_seq: 0,
            interval: Duration::from_secs(KEEPALIVE_INTERVAL_SECS),
            timeout: Duration::from_secs(KEEPALIVE_TIMEOUT_SECS),
        }
    }

    /// Check if it is time to send the next keepalive Ping.
    pub fn should_ping(&self) -> bool {
        self.last_sent_ping
            .map(|t| t.elapsed() >= self.interval)
            .unwrap_or(true)
    }

    /// Construct a Ping envelope and record its timestamp.
    pub fn next_ping(&mut self) -> SessionEnvelope {
        self.current_seq += 1;
        self.last_sent_ping = Some(Instant::now());

        SessionEnvelope {
            body: Some(Body::Ping(Ping {
                seq: self.current_seq,
            })),
        }
    }

    /// Construct a Pong envelope in response to a peer's Ping.
    pub fn make_pong(ping_seq: u64) -> SessionEnvelope {
        SessionEnvelope {
            body: Some(Body::Pong(Pong { seq: ping_seq })),
        }
    }

    /// Handle received Pong. Returns true if sequence matches.
    pub fn record_pong(&mut self, seq: u64) -> bool {
        self.last_received = Instant::now();
        seq == self.current_seq
    }

    /// Record receipt of any activity from the peer.
    pub fn record_activity(&mut self) {
        self.last_received = Instant::now();
    }

    /// Check if peer has timed out without responding.
    pub fn is_timed_out(&self) -> bool {
        self.last_received.elapsed() >= self.timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keepalive_ping_pong_flow() {
        let mut tracker = KeepaliveTracker::new();
        assert!(tracker.should_ping());

        let ping_env = tracker.next_ping();
        if let Some(Body::Ping(p)) = ping_env.body {
            assert_eq!(p.seq, 1);
            let pong_env = KeepaliveTracker::make_pong(p.seq);
            if let Some(Body::Pong(pong)) = pong_env.body {
                assert!(tracker.record_pong(pong.seq));
            } else {
                panic!("Expected Pong");
            }
        } else {
            panic!("Expected Ping");
        }

        assert!(!tracker.is_timed_out());
    }
}
