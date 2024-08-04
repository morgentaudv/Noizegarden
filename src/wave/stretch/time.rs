use derive_builder::Builder;

use crate::wave::sample::UniformedSample;

#[derive(Default, Debug, Clone, Copy, PartialEq, Builder)]
#[builder(default)]
pub struct TimeStretcher {
    /// サンプル区間のPeakとそれからのPeriodを計算するためのConvolutionに使うサンプル区間長さ
    pub template_size: usize,
    /// 0.5倍（伸びる）から100倍（縮む）まで指定可能。
    pub shrink_rate: f64,
    pub sample_period_min: usize,
    /// 必ず1以上であること。
    pub sample_period_length: usize,
}

pub struct TimeStretcherBufferSetting<'a> {
    pub buffer: &'a [UniformedSample],
}

impl TimeStretcher {
    pub fn process_with_buffer<'a>(&'a self, setting: &'a TimeStretcherBufferSetting) -> Option<Vec<UniformedSample>> {
        let original_buffer = setting.buffer;
        let original_samples_size = original_buffer.len();
        let shrink_rate = self.shrink_rate;

        let p_min = self.sample_period_min;
        let p_max = self.sample_period_min + self.sample_period_length;
        if p_min >= p_max {
            return None;
        }

        if shrink_rate < 0.5 || shrink_rate >= 100.0 {
            return None;
        }

        // そのままなら入力バッファをコピーするだけで終わる。
        if shrink_rate == 1.0 {
            return Some(original_buffer.to_owned());
        }

        let mut out_proceeded_endi = 0;
        let mut out_buffer = vec![];
        out_buffer.resize(1, UniformedSample::default());

        let mut offset_src = 0;
        let mut offset_dst = 0;
        while (offset_src + (p_max * 2)) < original_samples_size {
            // mサンプルずらして記入。一種のConvolutionを行う。
            // 一番Peak(正の数)が波形の周期だとみなす。
            let mut r_max = 0.0;
            let mut period = p_min;
            for m in p_min..=p_max {
                let mut result = 0.0;
                for n in 0..self.template_size {
                    let x_index = offset_src + n;
                    let y_index = offset_src + m + n;
                    result += original_buffer[x_index].to_f64() * original_buffer[y_index].to_f64();
                }

                if result > r_max {
                    r_max = result;
                    period = m;
                }
            }

            if shrink_rate >= 1.0 {
                // 元アルゴリズムとは違ってバッファを動的に用意する。
                // 複雑な波形の場合には固定のバッファだと枠がたりなくなる。
                out_proceeded_endi = offset_dst + period;
                if out_proceeded_endi >= out_buffer.len() {
                    let resize_len = (out_proceeded_endi + 1).next_power_of_two();
                    out_buffer.resize(resize_len, UniformedSample::default());
                }

                // 縮ませる
                for n in 0..period {
                    // 単調減少の重み付け。Lerpっぽくする。
                    let b_factor = (n as f64) / (period as f64);
                    let a_factor = 1.0 - b_factor;

                    let in_i = offset_dst + n;
                    out_buffer[in_i] = a_factor * original_buffer[offset_src + n];
                    out_buffer[in_i] += b_factor * original_buffer[offset_src + period + n];
                }

                let q_param = ((period as f64) / (shrink_rate - 1.0)).round() as usize;
                if q_param > period {
                    for n in period..q_param {
                        if offset_src + period + n >= original_samples_size {
                            break;
                        }

                        out_buffer[offset_dst + n] = original_buffer[offset_src + period + n];
                    }
                }

                offset_src += period + q_param;
                offset_dst += q_param;
            } else {
                // 元アルゴリズムとは違ってバッファを動的に用意する。
                // 複雑な波形の場合には固定のバッファだと枠がたりなくなる。
                out_proceeded_endi = offset_dst + (period * 2);
                if out_proceeded_endi >= out_buffer.len() {
                    let resize_len = (out_proceeded_endi + 1).next_power_of_two();
                    out_buffer.resize(resize_len, UniformedSample::default());
                }

                // 伸ばす
                for n in 0..period {
                    out_buffer[offset_dst + n] = original_buffer[offset_src + n];

                    // 単調減少の重み付け。Lerpっぽくする。
                    let b_factor = (n as f64) / (period as f64);
                    let a_factor = 1.0 - b_factor;

                    let out_i = offset_dst + period + n;
                    out_buffer[out_i] = a_factor * original_buffer[offset_src + period + n];
                    out_buffer[out_i] += b_factor * original_buffer[offset_src + n];
                }

                let q_param = ((period as f64 * shrink_rate) / (1.0 - shrink_rate)).round() as usize;
                if q_param > period {
                    for n in period..q_param {
                        if offset_src + n >= original_samples_size {
                            break;
                        }

                        // 動的に伸ばす。
                        let dst_len = offset_dst + period + n;
                        out_proceeded_endi = dst_len;
                        if out_proceeded_endi >= out_buffer.len() {
                            let resize_len = (out_proceeded_endi + 1).next_power_of_two();
                            out_buffer.resize(resize_len, UniformedSample::default());
                        }

                        out_buffer[dst_len] = original_buffer[offset_src + n];
                    }
                }

                offset_src += q_param;
                offset_dst += period + q_param;
            }
        }

        let _ = out_buffer.split_off(out_proceeded_endi);
        Some(out_buffer)
    }
}
