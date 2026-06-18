/// How often the displayed frame time is refreshed from the smoothed value.
const FRAME_TIME_REFRESH_INTERVAL: instant::Duration = instant::Duration::from_millis(500);

/// Tracks a smoothed frame time (exponential moving average) and exposes a value that only
/// refreshes every [`FRAME_TIME_REFRESH_INTERVAL`] so the on-screen readout stays readable.
pub struct FrameTimeTracker {
    smoothed_ms: f32,
    displayed_ms: f32,
    last_refresh: instant::Instant,
}

impl FrameTimeTracker {
    pub fn new() -> Self {
        Self {
            smoothed_ms: 0.0,
            displayed_ms: 0.0,
            last_refresh: instant::Instant::now(),
        }
    }

    /// Feeds the latest frame delta and returns the value to display, in milliseconds.
    pub fn update(&mut self, dt: instant::Duration) {
        let dt_ms = dt.as_secs_f32() * 1000.0;
        if self.smoothed_ms == 0.0 {
            // Seed on the first frame so we don't ramp up from zero.
            self.smoothed_ms = dt_ms;
            self.displayed_ms = dt_ms;
        } else {
            self.smoothed_ms += (dt_ms - self.smoothed_ms) * 0.1;
        }

        let now = instant::Instant::now();
        if now.duration_since(self.last_refresh) >= FRAME_TIME_REFRESH_INTERVAL {
            self.displayed_ms = self.smoothed_ms;
            self.last_refresh = now;
        }
    }

    pub fn displayed_ms(&self) -> f32 {
        self.displayed_ms
    }
}
