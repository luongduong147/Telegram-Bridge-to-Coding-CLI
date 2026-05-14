use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Clone, Debug)]
pub struct OpenCodeSession {
    pub name: String,
    pub workdir: String,
    pub last_activity: Instant,
}

impl OpenCodeSession {
    pub fn new(name: &str, workdir: &str) -> Self {
        Self {
            name: name.to_string(),
            workdir: workdir.to_string(),
            last_activity: Instant::now(),
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
