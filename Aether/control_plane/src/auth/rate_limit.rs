//! Login abuse protection — progressive backoff by IP and username.
//!
//! In-memory store suitable for single-process MVP. The [`LoginRateLimitStore`]
//! trait is the swap point for a future Redis-backed implementation.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::error::{Error, Result};

/// Maximum cooldown ceiling (5 minutes) — avoids permanent lockout DoS.
const MAX_COOLDOWN_SECS: u64 = 300;
/// Failures within this window contribute to progressive backoff.
const FAILURE_WINDOW: Duration = Duration::from_secs(15 * 60);
/// Threshold before first cooldown starts.
const FAILURE_THRESHOLD: u32 = 5;
/// Base cooldown after threshold (doubles with each additional failure).
const BASE_COOLDOWN_SECS: u64 = 30;

#[derive(Debug, Clone)]
struct AttemptState {
    failures: u32,
    first_failure: Instant,
    blocked_until: Option<Instant>,
}

impl Default for AttemptState {
    fn default() -> Self {
        Self {
            failures: 0,
            first_failure: Instant::now(),
            blocked_until: None,
        }
    }
}

/// Abstraction for login rate-limit storage (in-memory today, Redis later).
pub trait LoginRateLimitStore: Send + Sync {
    /// Returns Ok(()) if the attempt is allowed, Err(RateLimited) if blocked.
    fn check_allowed(&self, ip: &str, username: &str) -> Result<()>;

    /// Record a failed authentication attempt.
    fn record_failure(&self, ip: &str, username: &str);

    /// Reset counters after successful authentication.
    fn reset(&self, ip: &str, username: &str);
}

/// In-process rate limiter keyed by IP and by username (whichever is worse wins).
#[derive(Debug, Default)]
pub struct InMemoryLoginRateLimiter {
    by_ip: Mutex<HashMap<String, AttemptState>>,
    by_username: Mutex<HashMap<String, AttemptState>>,
}

impl InMemoryLoginRateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    fn check_map(map: &mut HashMap<String, AttemptState>, key: &str) -> Result<()> {
        let now = Instant::now();
        let entry = map.entry(key.to_string()).or_default();

        // Expire stale windows.
        if entry.failures > 0 && now.duration_since(entry.first_failure) > FAILURE_WINDOW {
            *entry = AttemptState::default();
        }

        if let Some(until) = entry.blocked_until {
            if now < until {
                return Err(Error::RateLimited("too many login attempts".into()));
            }
            // Cooldown elapsed — allow one attempt without clearing failure count
            // so repeated abuse escalates again.
            entry.blocked_until = None;
        }
        Ok(())
    }

    fn record_map(map: &mut HashMap<String, AttemptState>, key: &str) {
        let now = Instant::now();
        let entry = map.entry(key.to_string()).or_default();

        if entry.failures == 0 || now.duration_since(entry.first_failure) > FAILURE_WINDOW {
            *entry = AttemptState {
                failures: 1,
                first_failure: now,
                blocked_until: None,
            };
            return;
        }

        entry.failures = entry.failures.saturating_add(1);
        if entry.failures >= FAILURE_THRESHOLD {
            let excess = entry.failures.saturating_sub(FAILURE_THRESHOLD);
            let cooldown_secs =
                (BASE_COOLDOWN_SECS.saturating_mul(1u64 << excess.min(4))).min(MAX_COOLDOWN_SECS);
            entry.blocked_until = Some(now + Duration::from_secs(cooldown_secs));
        }
    }

    fn reset_map(map: &mut HashMap<String, AttemptState>, key: &str) {
        map.remove(key);
    }
}

impl LoginRateLimitStore for InMemoryLoginRateLimiter {
    fn check_allowed(&self, ip: &str, username: &str) -> Result<()> {
        let username_key = username.to_ascii_lowercase();
        {
            let mut by_ip = self
                .by_ip
                .lock()
                .map_err(|_| Error::Auth("rate limiter unavailable".into()))?;
            Self::check_map(&mut by_ip, ip)?;
        }
        {
            let mut by_user = self
                .by_username
                .lock()
                .map_err(|_| Error::Auth("rate limiter unavailable".into()))?;
            Self::check_map(&mut by_user, &username_key)?;
        }
        Ok(())
    }

    fn record_failure(&self, ip: &str, username: &str) {
        let username_key = username.to_ascii_lowercase();
        if let Ok(mut by_ip) = self.by_ip.lock() {
            Self::record_map(&mut by_ip, ip);
        }
        if let Ok(mut by_user) = self.by_username.lock() {
            Self::record_map(&mut by_user, &username_key);
        }
    }

    fn reset(&self, ip: &str, username: &str) {
        let username_key = username.to_ascii_lowercase();
        if let Ok(mut by_ip) = self.by_ip.lock() {
            Self::reset_map(&mut by_ip, ip);
        }
        if let Ok(mut by_user) = self.by_username.lock() {
            Self::reset_map(&mut by_user, &username_key);
        }
    }
}

/// Shared handle used by the Control Plane process.
pub type SharedLoginRateLimiter = Arc<InMemoryLoginRateLimiter>;
