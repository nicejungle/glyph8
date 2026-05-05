use std::time::{Duration, Instant};

/// Sliding-window FPS meter. Records a tick on each frame; reports the
/// instantaneous rate over the most recent `WINDOW` ticks.
pub struct FpsMeter {
    window: Vec<Instant>,
    capacity: usize,
}

impl FpsMeter {
    pub fn new(capacity: usize) -> Self {
        Self {
            window: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Record a tick (one rendered frame). Returns the current FPS
    /// estimate, or 0.0 if fewer than 2 ticks have been seen.
    pub fn tick(&mut self) -> f32 {
        let now = Instant::now();
        if self.window.len() == self.capacity {
            self.window.remove(0);
        }
        self.window.push(now);
        if self.window.len() < 2 {
            return 0.0;
        }
        let span: Duration = *self.window.last().unwrap() - self.window[0];
        if span.is_zero() {
            return 0.0;
        }
        (self.window.len() - 1) as f32 / span.as_secs_f32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn first_tick_returns_zero() {
        let mut m = FpsMeter::new(4);
        assert_eq!(m.tick(), 0.0);
    }

    #[test]
    fn estimates_roughly_60fps_when_ticks_are_16ms_apart() {
        let mut m = FpsMeter::new(8);
        // 5 ticks at ~16 ms intervals should give ~60 fps.
        m.tick();
        for _ in 0..4 {
            sleep(Duration::from_millis(16));
            let _ = m.tick();
        }
        let fps = m.tick();
        // Allow generous tolerance for sleep jitter, but it should be in the 50–80 range.
        assert!(fps > 40.0 && fps < 100.0, "expected ~60fps, got {}", fps);
    }
}
