use rand::random;
use std::{time::Duration, cmp::min, thread::sleep};

pub struct Backoff {
    limit: Duration,
    max_limit: Duration
}

impl Backoff {
    pub fn new(min: Duration, max: Duration) -> Self {
        Backoff { limit: min, max_limit: max }
    }
    pub fn backoff(&mut self) {
        let delay = random_duration(self.limit);
        self.limit = min(2 * self.limit, self.max_limit);
        sleep(delay);
    }
}

fn random_duration(limit: Duration) -> Duration {
    let nanos = random::<u64>() % limit.as_nanos() as u64;
    Duration::from_nanos(nanos)
}
