use crate::wave::sample::UniformedSample;

pub mod fir_lpf;
pub mod iir;

/// `input_buffer`から`filter_as`と`filter_bs`を使って
/// `output_buffer[write_sample_i]`にフィルタリングしたサンプルを記録する。
pub fn iir_compute_sample(
    output_i: usize,
    input_i: usize,
    output_buffer: &mut [UniformedSample],
    input_buffer: &[UniformedSample],
    filter_as: &[f64],
    filter_bs: &[f64],
) {
    debug_assert!(filter_as.len() == 3);
    debug_assert!(filter_bs.len() == 3);

    for ji in 0..=2 {
        if input_i < ji {
            break;
        }

        let bzxz = filter_bs[ji] * input_buffer[input_i - ji];
        output_buffer[output_i] += bzxz;
    }
    for ji in 1..=2 {
        if output_i < ji {
            break;
        }

        let azyz = filter_as[ji] * output_buffer[output_i - ji];
        output_buffer[output_i] -= azyz;
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------