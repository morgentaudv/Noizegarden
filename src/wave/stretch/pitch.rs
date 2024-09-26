use std::f64::consts::PI;

use derive_builder::Builder;

use crate::{
    math::sinc,
    wave::{sample::UniformedSample},
};
use crate::math::window::EWindowFunction;

/// `pitch_rate`にそって音源のプレイレートを変動しピッチをシフトする。
#[derive(Default, Debug, Clone, Copy, PartialEq, Builder)]
#[builder(default)]
pub struct PitchShifter {
    /// 0より大きく、また100以下の変動レート。
    /// 2倍だと周波数が2倍になる。（UEのMetaSoundなどは違って12微分音スケールではない)
    pub pitch_rate: f64,
    /// 2のべき乗であるべき。
    pub window_size: usize,
    /// Convolution後に補正するWindow関数のタイプ
    pub window_function: EWindowFunction,
}

pub struct PitchShifterBufferSetting<'a> {
    pub buffer: &'a [UniformedSample],
}

impl PitchShifter {
    /// 処理関数。
    pub fn process_with_buffer<'a>(&'a self, setting: &'a PitchShifterBufferSetting) -> Option<Vec<UniformedSample>> {
        let pitch_rate = self.pitch_rate;
        if pitch_rate <= 0.0 || pitch_rate >= 100.0 {
            return None;
        }

        let window_size = self.window_size;
        if window_size == 0 || !window_size.is_power_of_two() {
            return None;
        }

        let src_buffer = setting.buffer;
        let window_half = window_size >> 1;
        let window_function = self.window_function;

        let mut dst_buffer = vec![];
        let dst_samples_size = ((src_buffer.len() as f64) / pitch_rate).ceil() as usize;
        dst_buffer.resize(dst_samples_size, UniformedSample::default());

        for n in 0..dst_samples_size {
            let t = pitch_rate * (n as f64);
            let ta = t.floor() as usize;

            let mut tb = 0usize;
            if t == (ta as f64) {
                tb = ta;
            } else {
                tb = ta + 1;
            }

            let hann_src = (tb as isize) - (window_half as isize);
            let hann_dst = (ta as isize) + (window_half as isize);
            let hann_length = hann_dst - hann_src;

            let window_src = if tb >= window_half { tb - window_half } else { 0 };
            let window_dst = (ta + window_half).min(src_buffer.len());
            if window_src < window_dst {
                for m in window_src..window_dst {
                    // ここでConvolution。
                    // s_d(m)sinc(pi(t-m))

                    let hann_value = window_function.get_factor(hann_length as f64, t - (m as f64));
                    let sinc_value = sinc((PI as f64) * (t - (m as f64)));
                    dst_buffer[n] += (hann_value * sinc_value) * src_buffer[m];
                }
            }
        }

        Some(dst_buffer)
    }
}
