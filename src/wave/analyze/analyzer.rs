use derive_builder::Builder;
use itertools::Itertools;

use crate::wave::{complex::Complex, container::WaveContainer, sample::UniformedSample, PI2};

use super::{sine_freq::SineFrequency, window::EWindowFunction, EAnalyzeMethod};

/// AnalyzerV2。
#[derive(Default, Debug, Clone, Copy, PartialEq, Builder)]
#[builder(default)]
pub struct FrequencyAnalyzerV2 {
    /// 波形の分析モード。DFTまたはFFTか。
    pub analyze_method: EAnalyzeMethod,
    /// 分析する周波数のスタート。
    /// 必ず0か正の数であるべき。
    pub frequency_start: f64,
    /// 分析する周波数の範囲、帯域幅。
    /// 必ず0より大きい数値を入れるべき。
    pub frequency_width: f64,
    /// 分析する周波数帯域幅を分割する数。分析周波数ビンの数。
    /// 必ず1以上いれるべき。
    /// もしFFTなら、2のべき乗であって`WaveContainerSetting`の`samples_count`と一致すべき。
    pub frequency_bin_count: u32,
    /// 窓関数の指定。
    pub window_function: EWindowFunction,
}

/// 波形コンテナ関連の設定を記載。
pub struct WaveContainerSetting<'a> {
    pub container: &'a WaveContainer,
    pub start_sample_index: usize,
    pub samples_count: usize,
}

impl FrequencyAnalyzerV2 {
    pub fn analyze_container<'a>(&'a self, _container: &'a WaveContainerSetting) -> Option<Vec<SineFrequency>> {
        None
    }
}

// ----------------------------------------------------------------------------
//
// DEPRECATED DEPRECATED DEPRECATED DEPRECATED DEPRECATED
// DEPRECATED DEPRECATED DEPRECATED DEPRECATED DEPRECATED
// DEPRECATED DEPRECATED DEPRECATED DEPRECATED DEPRECATED
// DEPRECATED DEPRECATED DEPRECATED DEPRECATED DEPRECATED
// DEPRECATED DEPRECATED DEPRECATED DEPRECATED DEPRECATED
//
// ----------------------------------------------------------------------------

///
#[derive(Debug, Default, Clone, Copy, Builder)]
#[builder(default)]
pub struct FrequencyAnalyzer {
    pub start_sample_index: usize,
    pub frequency_start: f64,
    pub sample_rate: u32,
    pub samples_count: usize,
    pub window_function: Option<EWindowFunction>,
    pub analyze_method: EAnalyzeMethod,
}

impl FrequencyAnalyzer {
    /// [`WaveContainer`]の波形から周波数特性を計算する。
    pub fn analyze_container(&self, container: &WaveContainer) -> Option<Vec<SineFrequency>> {
        // まず入れられた情報から範囲に収められそうなのかを確認する。
        // sound_lengthはhalf-opened rangeなのかclosedなのかがいかがわしい模様。
        let wave_sound_length = container.sound_length() as f64;
        let recip_sample_per_sec = (container.samples_per_second() as f64).recip();
        let samples_time_length = recip_sample_per_sec * (self.samples_count as f64);
        let samples_time_start = (self.start_sample_index as f64) * recip_sample_per_sec;
        let samples_time_end = samples_time_start + samples_time_length;

        if samples_time_end > wave_sound_length {
            return None;
        }

        // [time_start, time_start + sampels_time_end)の時間領域を
        // [frequency_start, frequency_start + frequency_length]まで分析する。
        //
        // 現在はfrequency_lengthはsamples_countと同様。
        if self.frequency_start < 0.0 || self.samples_count <= 0 {
            return None;
        }
        assert!(container.channel() == 1);

        match self.analyze_method {
            EAnalyzeMethod::DFT => Some(analyze_as_dft(self, container.uniformed_sample_buffer())),
            EAnalyzeMethod::FFT => {
                if !self.samples_count.is_power_of_two() {
                    None
                } else {
                    Some(analyze_as_fft(self, container.uniformed_sample_buffer()))
                }
            }
        }
    }

    /// [`UniformedSample`]のある任意の波形バッファから周波数特性を計算する。
    pub fn analyze_sample_buffer(&self, sample_buffer: &[UniformedSample]) -> Option<Vec<SineFrequency>> {
        // まずsampleの尺が足りるかを確認する。
        if sample_buffer.len() < self.samples_count {
            return None;
        }
        if self.frequency_start < 0.0 || self.samples_count <= 0 {
            return None;
        }

        match self.analyze_method {
            EAnalyzeMethod::DFT => Some(analyze_as_dft(self, sample_buffer)),
            EAnalyzeMethod::FFT => {
                if !self.samples_count.is_power_of_two() {
                    None
                } else {
                    Some(analyze_as_fft(self, sample_buffer))
                }
            }
        }
    }
}

impl FrequencyAnalyzer {
    ///
    fn get_window_fn_factor(&self, length: f64, time: f64) -> f64 {
        if let Some(window_fn) = self.window_function {
            window_fn.get_factor(length, time)
        } else {
            1f64
        }
    }
}

/// [`Discreted Fourier Transform`](https://en.wikipedia.org/wiki/Discrete_Fourier_transform)（離散フーリエ変換）を行って
/// 周波数特性を計算して返す。
fn analyze_as_dft(analyzer: &FrequencyAnalyzer, sample_buffer: &[UniformedSample]) -> Vec<SineFrequency> {
    assert!(analyzer.samples_count > 0);

    let freq_precision = match analyzer.sample_rate {
        0 => 1.0,
        _ => (analyzer.sample_rate as f64) / (analyzer.samples_count as f64),
    };

    let mut results = vec![];
    let mut cursor_frequency = analyzer.frequency_start;
    let valid_sample_counts = analyzer.samples_count >> 1;

    // ナイキスト周波数の半分まで取る。
    for _ in 0..valid_sample_counts {
        let mut frequency_response = Complex::<f64>::default();

        for local_i in 0..analyzer.samples_count {
            // アナログ波形に複素数の部分は存在しないので、Realパートだけ扱う。
            // coeff_input = exp(2pifn / N)
            let time_factor = (local_i as f64) / (analyzer.samples_count as f64);
            let coeff_input = PI2 * cursor_frequency * time_factor;
            let coefficient = Complex::<f64>::from_exp(coeff_input * -1.0);

            let sample = {
                let sample_i = local_i + analyzer.start_sample_index;
                let amplitude = sample_buffer[sample_i].to_f64();
                let window_factor = analyzer.get_window_fn_factor(1.0, time_factor);
                amplitude * window_factor
            };
            frequency_response += sample * coefficient;
        }

        results.push(SineFrequency::from_complex_f64(cursor_frequency, frequency_response));

        // 周波数カーソルを進める。
        cursor_frequency += freq_precision;
    }

    results
}

/// [`Fast Fourier Transform`](https://en.wikipedia.org/wiki/Fast_Fourier_transform)（高速フーリエ変換）を行って
/// 周波数特性を計算して返す。
fn analyze_as_fft(analyzer: &FrequencyAnalyzer, sample_buffer: &[UniformedSample]) -> Vec<SineFrequency> {
    assert!(analyzer.samples_count.is_power_of_two());

    // まず最後に求められる各Frequencyの情報をちゃんとした位置に入れるためのIndexルックアップテーブルを作る。
    // たとえば、index_count = 8のときに1番目のFrequency情報は4番目に入れるべきなど…
    let lookup_table = {
        // ビットリバーステクニックを使ってテーブルを作成。
        let mut results = vec![0];
        let mut addition_count = analyzer.samples_count >> 1;
        while addition_count > 0 {
            results.append(&mut results.iter().map(|v| v + addition_count).collect_vec());
            addition_count >>= 1;
        }

        results
    };
    let samples_count = analyzer.samples_count;

    // まず最後レベルの信号を計算する。index_count分作る。
    let final_signals = {
        let mut prev_signals: Vec<Complex<f64>> = vec![];
        prev_signals.reserve(samples_count);

        // 無限に伸びる周期波形をつくるよりは、すでに与えられた波形をもっと細かく刻んでサンプルしたほうが安定そう。
        for local_i in 0..samples_count {
            // アナログ波形に複素数の部分は存在しないので、Realパートだけ扱う。
            let amplitude = {
                let sample_i = local_i + analyzer.start_sample_index;
                let signal = sample_buffer[sample_i].to_f64();

                let time_factor = (local_i as f64) / (samples_count as f64);
                let window_factor = analyzer.get_window_fn_factor(1.0, time_factor);
                signal * window_factor
            };

            // 負の数のAmplitudeも可能。
            prev_signals.push(Complex::<f64> {
                real: amplitude,
                imag: 0.0,
            });
        }

        //
        let mut next_signals: Vec<Complex<f64>> = vec![];
        next_signals.resize(analyzer.samples_count, <Complex<f64> as Default>::default());

        let level = (samples_count as f64).log2().ceil() as usize;
        for lv_i in 0..level {
            let index_period = samples_count >> lv_i;
            let half_index = index_period >> 1;

            for period_i in (0..samples_count).step_by(index_period) {
                for local_i in 0..half_index {
                    let lhs_i = period_i + local_i;
                    let rhs_i = period_i + local_i + half_index;
                    let prev_lhs_signal = prev_signals[lhs_i];
                    let prev_rhs_signal = prev_signals[rhs_i];
                    let coefficient =
                        Complex::<f64>::from_exp(PI2 * (local_i as f64) / (index_period as f64)).conjugate();

                    let new_lhs_signal = prev_lhs_signal + prev_rhs_signal;
                    let new_rhs_signal = coefficient * (prev_lhs_signal - prev_rhs_signal);
                    next_signals[lhs_i] = new_lhs_signal;
                    next_signals[rhs_i] = new_rhs_signal;
                }
            }

            // 次のレベルでprev→nextをするためにswapする。
            std::mem::swap(&mut prev_signals, &mut next_signals);
        }

        prev_signals
    };

    // 計算済みの`final_signals`はビットリバースのシグナルリストに１対１対応しているので
    // このままルックアップテーブルから結果シグナルに入れて[`SineFrequency`]に変換して返す。
    let mut results = vec![];
    results.resize(samples_count, SineFrequency::default());

    let freq_precision = 1.0;
    for freq_i in 0..samples_count {
        let target_i = lookup_table[freq_i];

        let frequency = analyzer.frequency_start + (freq_precision * (target_i as f64));
        let sine_freq = SineFrequency::from_complex_f64(frequency, final_signals[freq_i]);
        results[target_i] = sine_freq;
    }

    results
}
