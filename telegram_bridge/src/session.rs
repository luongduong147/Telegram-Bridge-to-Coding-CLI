use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Clone, Debug)]
pub struct OpenCodeSession {
    pub last_activity: Instant,
    pub active_cli_name: String,
    pub active_workdir_index: usize,
}

impl OpenCodeSession {
    pub fn new() -> Self {
        Self {
            last_activity: Instant::now(),
            active_cli_name: String::new(),
            active_workdir_index: 0,
        }
    }

    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn is_expired(&self) -> bool {
        self.last_activity.elapsed() > TIMEOUT
    }

    pub fn remaining(&self) -> Duration {
        TIMEOUT
            .checked_sub(self.last_activity.elapsed())
            .unwrap_or(Duration::ZERO)
    }
}
