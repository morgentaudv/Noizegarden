use crate::wave::sample::UniformedSample;

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
