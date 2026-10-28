//! [`Clock`] reading the system time.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::application::ports::Clock;

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}
