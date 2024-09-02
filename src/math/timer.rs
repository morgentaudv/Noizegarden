use std::time;

/// 時間測定に使うもの。
pub struct Timer {
    fixed_duration: time::Duration,
    previous_time: time::Instant,
}

impl Timer {
    /// Create timer instance with fixed duration as a second unit.
    pub fn from_second(tick_second: f64) -> Self {
        Timer {
            fixed_duration: time::Duration::from_secs_f64(tick_second),
            previous_time: time::Instant::now(),
        }
    }

    /// Tick timer and update variables, return true if ticked.
    /// Otherwise, return false.
    pub fn fixed_tick(&mut self) -> bool {
        let now_time = time::Instant::now();
        let elapsed = now_time.duration_since(self.previous_time);
        if elapsed < self.fixed_duration {
            false
        } else {
            self.previous_time = now_time;
            true
        }
    }

    /// 動的にTickする。前のTickから経った時間を[`time::Duration`]で返す。
    pub fn tick(&mut self) -> time::Duration {
        let now_time = time::Instant::now();
        let elapsed = now_time.duration_since(self.previous_time);
        self.previous_time = now_time;
        elapsed
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
