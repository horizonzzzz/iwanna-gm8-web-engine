use crate::{RuntimeTimeHost, DEFAULT_TICK_RATE_HZ};

#[derive(Debug, Clone, Copy)]
pub struct DeterministicClock {
    now_nanos: u128,
    tick_rate_hz: u32,
}

impl DeterministicClock {
    pub fn new(start_nanos: u128, tick_rate_hz: u32) -> Self {
        Self {
            now_nanos: start_nanos,
            tick_rate_hz,
        }
    }

    pub fn advance_frames(&mut self, frames: u64) {
        if self.tick_rate_hz == 0 {
            return;
        }

        let frame_nanos = 1_000_000_000u128 / u128::from(self.tick_rate_hz);
        self.now_nanos += frame_nanos.saturating_mul(u128::from(frames));
    }
}

impl Default for DeterministicClock {
    fn default() -> Self {
        Self::new(0, DEFAULT_TICK_RATE_HZ)
    }
}

impl RuntimeTimeHost for DeterministicClock {
    fn now_nanos(&self) -> u128 {
        self.now_nanos
    }

    fn tick_rate_hz(&self) -> u32 {
        self.tick_rate_hz
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_clock_advances_by_frame_count() {
        let mut clock = DeterministicClock::new(0, 50);
        clock.advance_frames(3);

        assert_eq!(clock.now_nanos(), 60_000_000);
        assert_eq!(clock.tick_rate_hz(), 50);
    }
}
