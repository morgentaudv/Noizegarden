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

pub(super) struct AmplitudeADSRInternal {
    pub attack_sample_len: usize,
    pub decay_sample_len: usize,
    pub sustain_intensity: f64,
    pub release_sample_len: usize,
    pub gate_sample_len: usize,
    pub duration_sample_len: usize,
}

impl AmplitudeADSRInternal {
    fn compute_intensity(&self, sample_i: usize) -> f64 {
        match sample_i {
            x if (0..self.attack_sample_len).contains(&x) => match self.attack_sample_len {
                0 => 1.0,
                _ => 1.0 - (-5.0 * (sample_i as f64) / (self.attack_sample_len as f64)).exp(),
            },
            x if (self.attack_sample_len..self.gate_sample_len).contains(&x) => {
                let s = self.sustain_intensity;
                match self.decay_sample_len {
                    0 => s,
                    _ => {
                        let off = sample_i - self.attack_sample_len;
                        s + ((1.0 - s) * (-5.0 * (off as f64) / (self.decay_sample_len as f64)).exp())
                    }
                }
            }
            x if (self.gate_sample_len..self.duration_sample_len).contains(&x) => match self.release_sample_len {
                0 => 0.0,
                _ => {
                    let e = self.compute_intensity(self.gate_sample_len - 1);
                    let off = sample_i - self.gate_sample_len + 1;
                    e * (-5.0 * (off as f64) / (self.release_sample_len as f64)).exp()
                }
            },
            _ => 1.0,
        }
    }

    /// Apply ADSR(Attack - Decay - Sustain - Release) to amplitude.
    pub(super) fn apply(&self, read_buffer: &[UniformedSample]) -> Vec<UniformedSample> {
        assert!(self.gate_sample_len >= 1);
        assert!(self.attack_sample_len + self.decay_sample_len <= self.gate_sample_len);
        assert!(self.gate_sample_len <= self.duration_sample_len);

        let buffer_length = read_buffer.len();
        let mut buffer = vec![UniformedSample::default(); buffer_length];

        for sample_i in 0..buffer_length {
            let amp_mul_factor = self.compute_intensity(sample_i);
            buffer[sample_i] = amp_mul_factor * read_buffer[sample_i];
        }

        buffer
    }
}
