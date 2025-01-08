use crate::math::get_required_sample_count;

///
#[derive(Debug, Clone)]
pub struct SampleTimer {
    internal_time: f64,
    /// サンプルを取得するための最後に処理した時間
    last_process_time: f64,
}

impl SampleTimer {
    pub fn new(initial_time: f64) -> Self {
        Self {
            internal_time: initial_time,
            last_process_time: initial_time,
        }
    }

    pub fn internal_time(&self) -> f64 {
        self.internal_time
    }

    pub fn process_time(&mut self, frame_time: f64, sample_rate: usize) -> ProcessTimeResult {
        if sample_rate == 0 {
            return ProcessTimeResult {
                required_sample_count: 0,
                old_time: self.last_process_time,
            };
        }

        self.internal_time += frame_time;
        let time_offset = self.internal_time - self.last_process_time;
        let sample_counts = get_required_sample_count(time_offset, sample_rate);
        if sample_counts <= 0 {
            return ProcessTimeResult {
                required_sample_count: 0,
                old_time: self.last_process_time,
            };
        }

        // タイマーがまだ動作前なら何もしない。
        let old_internal_time = self.last_process_time;
        self.last_process_time = self.internal_time;
        ProcessTimeResult {
            required_sample_count: sample_counts,
            old_time: old_internal_time,
        }
    }
}

///
#[derive(Default, Debug, Clone, Copy)]
pub struct ProcessTimeResult {
    pub required_sample_count: usize,
    pub old_time: f64,
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
