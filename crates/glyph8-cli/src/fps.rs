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
        self.tick_at(Instant::now())
    }

    /// Like [`tick`](Self::tick), but uses the supplied instant. Lets tests
    /// drive the meter with synthetic timestamps instead of wall-clock sleep.
    pub fn tick_at(&mut self, now: Instant) -> f32 {
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

    #[test]
    fn first_tick_returns_zero() {
        let mut m = FpsMeter::new(4);
        assert_eq!(m.tick(), 0.0);
    }

    #[test]
    fn estimates_60fps_when_ticks_are_16_667ms_apart() {
        let mut m = FpsMeter::new(8);
        let t0 = Instant::now();
        // 6 synthetic ticks exactly 1/60 s apart -> exactly 60 fps.
        let step = Duration::from_nanos(16_666_667);
        for i in 0..6 {
            let _ = m.tick_at(t0 + step * i);
        }
        let fps = m.tick_at(t0 + step * 6);
        assert!((fps - 60.0).abs() < 0.1, "expected ~60fps, got {fps}");
    }

    #[test]
    fn window_slides_when_capacity_exceeded() {
        let mut m = FpsMeter::new(3);
        let t0 = Instant::now();
        let step = Duration::from_millis(10);
        // 5 ticks, capacity 3 -> only the last 3 timestamps survive,
        // so the FPS estimate reflects the recent rate (100 fps), not the average since t0.
        for i in 0..5 {
            let _ = m.tick_at(t0 + step * i);
        }
        let fps = m.tick_at(t0 + step * 5);
        assert!((fps - 100.0).abs() < 0.5, "expected ~100fps, got {fps}");
    }
}
