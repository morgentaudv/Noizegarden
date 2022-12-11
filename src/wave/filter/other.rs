use crate::wave::{sample::UniformedSample, PI2};

pub(super) struct DeemphasizerInternal {
    pub coefficient: f64,
}

impl DeemphasizerInternal {
    pub(super) fn apply(&self, read_buffer: &[UniformedSample]) -> Vec<UniformedSample> {
        let buffer_length = read_buffer.len();
        let mut buffer = vec![UniformedSample::default(); buffer_length];

        buffer[0] = read_buffer[0];
        for sample_i in 1..buffer_length {
            buffer[sample_i] = read_buffer[sample_i] + (self.coefficient * buffer[sample_i - 1]);
        }

        buffer
    }
}

pub(super) struct PreEmphasizerInternal {
    pub coefficient: f64,
}

impl PreEmphasizerInternal {
    pub(super) fn apply(&self, read_buffer: &[UniformedSample]) -> Vec<UniformedSample> {
        let buffer_length = read_buffer.len();
        let mut buffer = vec![UniformedSample::default(); buffer_length];

        buffer[0] = read_buffer[0];
        for sample_i in 1..buffer_length {
            buffer[sample_i] = read_buffer[sample_i] - (self.coefficient * read_buffer[sample_i - 1]);
        }

        buffer
    }
}

/// [`super::ESourceFilter`]の内部処理用の構造体。
/// Crateの内部でしか接近できない。
pub(super) struct AmplitudeTremoloInternal {
    pub initial_scale: f64,
    pub periodical_scale_factor: f64,
    pub period_time_frequency: f64,
    pub source_samples_per_second: f64,
}

impl AmplitudeTremoloInternal {
    /// LFO (Low Frequency Oscillator)を使ってVCAに振幅の時間エンベロープを適用する。
    pub(super) fn apply(&self, read_buffer: &[UniformedSample]) -> Vec<UniformedSample> {
        let buffer_length = read_buffer.len();
        let mut buffer = vec![UniformedSample::default(); buffer_length];

        let coefficient = PI2 * self.period_time_frequency / self.source_samples_per_second;

        for sample_i in 0..buffer_length {
            let amp_mul_factor =
                self.initial_scale + (self.periodical_scale_factor * (coefficient * (sample_i as f64)).sin());
            buffer[sample_i] = amp_mul_factor * read_buffer[sample_i];
        }

        buffer
    }
}
