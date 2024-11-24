use crate::math::sinc;
use crate::wave::sample::UniformedSample;
use crate::wave::PI2;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

pub mod fir;
pub mod iir;
pub mod irconv;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum EFilterMode {
    #[serde(rename = "low-pass")]
    LowPass,
    #[serde(rename = "high-pass")]
    HighPass,
    #[serde(rename = "band-pass")]
    BandPass,
    #[serde(rename = "band-stop")]
    BandStop,
}

/// FIRの応答計算。
pub fn compute_fir_response(filters_count: usize, edge: f64, width: f64, mode: EFilterMode) -> Vec<f64> {
    // isizeに変更する理由としては、responseを計算する際に負の数のIndexにも接近するため
    let filters_count = filters_count as isize;

    // -filters_count/2からfilters_count/2までにEWindowFunction(Hann)の値リストを求める。
    let windows = (0..=filters_count)
        .map(|v| {
            let sine = PI2 * ((v as f64) + 0.5) / ((filters_count + 1) as f64);
            (1.0 - sine) * 0.5
        })
        .collect_vec();

    // フィルタ係数の週はす特性bを計算する。
    let mut bs = (((filters_count >> 1) * -1)..=(filters_count >> 1))
        .map(|v| {
            let v = v as f64;
            match mode {
                EFilterMode::LowPass => 2.0 * edge * sinc(PI2 * edge * v),
                EFilterMode::HighPass => sinc(PI * v) - 2.0 * edge * sinc(PI2 * edge * v),
                EFilterMode::BandPass => {
                    let e1 = edge - (width * 0.5);
                    let e2 = edge + (width * 0.5);

                    2.0 * ((e2 * sinc(PI2 * e2 * v)) - (e1 * sinc(PI2 * e1 * v)))
                }
                EFilterMode::BandStop => {
                    let e1 = edge - (width * 0.5);
                    let e2 = edge + (width * 0.5);
                    let r = 2.0 * ((e2 * sinc(PI2 * e2 * v)) - (e1 * sinc(PI2 * e1 * v)));

                    sinc(PI * v) - r
                }
            }
        })
        .collect_vec();

    assert_eq!(bs.len(), windows.len());
    for i in 0..windows.len() {
        bs[i] *= windows[i];
    }
    bs
}

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
