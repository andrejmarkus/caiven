use std::time::Duration;

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
        while self.accumulator >= self.timestep {
            self.accumulator -= self.timestep;
            steps += 1;
        }
        steps
    }
}
