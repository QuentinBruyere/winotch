//! Elapsed time that can be started, paused and reset: shared by the
//! stopwatch and the timer modules. Pure logic (time is passed in), tested.

use std::time::{Duration, Instant};

#[derive(Debug, Default, Clone, Copy)]
pub struct Watch {
    /// Time counted before the current run.
    counted: Duration,
    /// Start of the current run, while running.
    since: Option<Instant>,
}

impl Watch {
    pub fn elapsed(&self, now: Instant) -> Duration {
        self.counted
            + self
                .since
                .map_or(Duration::ZERO, |s| now.saturating_duration_since(s))
    }

    pub fn running(&self) -> bool {
        self.since.is_some()
    }

    /// Never started since the last reset.
    pub fn idle(&self) -> bool {
        !self.running() && self.counted.is_zero()
    }

    pub fn start(&mut self, now: Instant) {
        if self.since.is_none() {
            self.since = Some(now);
        }
    }

    pub fn pause(&mut self, now: Instant) {
        if self.since.is_some() {
            self.counted = self.elapsed(now);
            self.since = None;
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// How long until the shown whole seconds change, while running.
    pub fn until_next_second(&self, now: Instant) -> Option<Duration> {
        self.running().then(|| {
            let nanos = self.elapsed(now).subsec_nanos();
            Duration::from_nanos(u64::from(1_000_000_000 - nanos))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_only_while_running() {
        let t0 = Instant::now();
        let s = Duration::from_secs;
        let mut watch = Watch::default();
        assert!(watch.idle());
        watch.start(t0);
        assert_eq!(watch.elapsed(t0 + s(10)), s(10));
        watch.pause(t0 + s(10));
        assert!(!watch.idle() && !watch.running());
        assert_eq!(watch.elapsed(t0 + s(100)), s(10));
        watch.start(t0 + s(100));
        assert_eq!(watch.elapsed(t0 + s(105)), s(15));
        watch.reset();
        assert!(watch.idle());
        assert_eq!(watch.elapsed(t0 + s(200)), Duration::ZERO);
    }

    #[test]
    fn starting_twice_keeps_the_first_start() {
        let t0 = Instant::now();
        let mut watch = Watch::default();
        watch.start(t0);
        watch.start(t0 + Duration::from_secs(5));
        assert_eq!(
            watch.elapsed(t0 + Duration::from_secs(8)),
            Duration::from_secs(8)
        );
    }
}
