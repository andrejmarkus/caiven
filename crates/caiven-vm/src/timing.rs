use std::time::Duration;

/// Steps `tick` will return in one call before it gives up on catching up
/// and discards the rest of the accumulated time instead. A long stall
/// (window minimized, debugger breakpoint, host machine suspended) would
/// otherwise bank minutes of `dt`, and the very next `tick` would hand back
/// that whole gap as one enormous burst of catch-up frames — spending real
/// wall-clock seconds just running the VM, which reads to a player as the
/// console having hung.
const MAX_CATCH_UP_STEPS: u32 = 5;

pub struct FixedTimestep {
    accumulator: Duration,
    timestep: Duration,
}

impl FixedTimestep {
    pub fn new(hz: u32) -> Self {
        Self {
            accumulator: Duration::ZERO,
            timestep: Duration::from_secs(1) / hz,
        }
    }

    pub fn tick(&mut self, dt: Duration) -> u32 {
        self.accumulator += dt;
        let mut steps = 0u32;
        while steps < MAX_CATCH_UP_STEPS && self.accumulator >= self.timestep {
            self.accumulator -= self.timestep;
            steps += 1;
        }
        if steps == MAX_CATCH_UP_STEPS {
            self.accumulator = Duration::ZERO;
        }
        steps
    }

    /// Drops any banked time. Callers must use this after a gap where
    /// `tick` wasn't called for real elapsed time (e.g. the VM was paused)
    /// — otherwise the next `tick` sees the whole gap as `dt` and dumps it
    /// as one huge burst of catch-up steps.
    pub fn reset(&mut self) {
        self.accumulator = Duration::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_stall_does_not_dump_a_huge_catch_up_burst() {
        let mut timing = FixedTimestep::new(60);
        // A naive accumulator would report ~216,000 steps for an hour-long
        // stall (window minimized, debugger breakpoint, host suspended) in
        // this one call — each of which the caller would then have to
        // actually run.
        let steps = timing.tick(Duration::from_secs(3600));
        assert!(
            steps <= MAX_CATCH_UP_STEPS,
            "expected steps capped at {MAX_CATCH_UP_STEPS}, got {steps}"
        );
    }

    #[test]
    fn a_capped_stall_discards_the_rest_of_the_backlog() {
        let mut timing = FixedTimestep::new(60);
        timing.tick(Duration::from_secs(3600));
        // If the leftover backlog weren't discarded, the very next tick —
        // even with no further elapsed time — would still report a full
        // batch, spreading the burst across extra calls instead of
        // actually dropping it.
        let steps = timing.tick(Duration::ZERO);
        assert_eq!(steps, 0);
    }

    #[test]
    fn ordinary_frame_pacing_is_unaffected() {
        let mut timing = FixedTimestep::new(60);
        assert_eq!(timing.tick(Duration::from_secs_f64(1.0 / 60.0)), 1);
        assert_eq!(timing.tick(Duration::from_secs_f64(1.0 / 60.0)), 1);
        assert_eq!(timing.tick(Duration::from_secs_f64(2.0 / 60.0)), 2);
    }
}
