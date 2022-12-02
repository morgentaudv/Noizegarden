use derive_builder::Builder;
use itertools::Itertools;

use super::complex::Complex;
use super::container::WaveContainer;
use super::PI2;

/// 窓関数（Windowing Function）の種類の値を持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EWindowFunction {
    /// ハン窓関数を適用する。
    Hann,
}

impl EWindowFunction {
    /// 掛け算数値を計算する。もし範囲外なら、0だけを返す。
    pub fn get_factor(&self, start: f64, length: f64, now: f64) -> f64 {
        // もし範囲外なら0を返す。
        if now < start || (now - start) > length {
            return 0f64;
        }

        let t = ((now - start) / length).clamp(0.0, 1.0);
        match self {
            EWindowFunction::Hann => {
                // 中央が一番高く、両端が0に収束する。
                (1f64 - (PI2 * t).cos()) * 0.5f64
            }
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EAnalyzeMethod {
    #[default]
    DFT,
    FFT,
}

///
#[derive(Debug, Default, Clone, Copy, Builder)]
#[builder(default)]
pub struct FrequencyAnalyzer {
    pub time_start: f64,
    pub time_length: f64,
    pub time_precision: f64,
    pub frequency_start: f64,
    pub frequency_length: f64,
    pub frequency_precision: f64,
    pub window_function: Option<EWindowFunction>,
    pub analyze_method: EAnalyzeMethod,
}

impl FrequencyAnalyzer {
    /// 周波数特性を計算する。
    pub fn analyze_frequencies(&self, container: &WaveContainer) -> Option<Vec<SineFrequency>> {
        // まず入れられた情報から範囲に収められそうなのかを確認する。
        // sound_lengthはhalf-opened rangeなのかclosedなのかがいかがわしい模様。
        let sound_length = container.sound_length() as f64;
        let time_end = self.time_start + self.time_length;
        if time_end > sound_length {
            return None;
        }

        // [freuqnecy_start, start + frequency_length)までに分析する。
        if self.frequency_length <= 0.0 || self.frequency_start < 0.0 || self.frequency_precision <= 0.0 {
            return None;
        }

        match self.analyze_method {
            EAnalyzeMethod::DFT => Some(self.analyze_as_dft(container)),
            EAnalyzeMethod::FFT => Some(self.analyze_as_fft(container)),
        }
    }

    /// [`Discreted Fourier Transform`](https://en.wikipedia.org/wiki/Discrete_Fourier_transform)（離散フーリエ変換）を行って
    /// 周波数特性を計算して返す。
    fn analyze_as_dft(&self, container: &WaveContainer) -> Vec<SineFrequency> {
        let frequency_end = self.frequency_start + self.frequency_length;
        let time_end = self.time_start + self.time_length;

        let mut results = vec![];
        let mut cursor_frequency = self.frequency_start;
        while cursor_frequency < frequency_end {
            let mut cursor_time = self.time_start;
            let mut frequency = Complex::<f64>::default();

            while cursor_time < time_end {
                // アナログ波形に複素数の部分は存在しないので、Realパートだけ扱う。
                let time_factor = ((cursor_time - self.time_start) / self.time_length).clamp(0.0, 1.0);
                let coeff_input = PI2 * cursor_frequency * time_factor;
                let coefficient = Complex::<f64>::from_exp(coeff_input).conjugate();

                let sample = {
                    let amplitude = container.uniform_sample_of_f64(cursor_time).unwrap().to_f64();
                    let window_factor = self.get_window_fn_factor(cursor_time);
                    amplitude * window_factor
                };
                frequency += sample * coefficient;

                // 時間カーソルを進める。
                cursor_time += self.time_precision;
            }

            results.push(SineFrequency::from_complex_f64(cursor_frequency, frequency));

            // 周波数カーソルを進める。
            cursor_frequency += self.frequency_precision;
        }

        results
    }

    /// [`Fast Fourier Transform`](https://en.wikipedia.org/wiki/Fast_Fourier_transform)（高速フーリエ変換）を行って
    /// 周波数特性を計算して返す。
    fn analyze_as_fft(&self, container: &WaveContainer) -> Vec<SineFrequency> {
        let frequency_end = self.frequency_start + self.frequency_length;
        let time_end = self.time_start + self.time_length;

        // まずfrequencyのインデックスとtimeのインデックスから大きいインデックスを取り、
        // 一番近いながらも大きい2のべき乗の値に実際に使うインデックスとして使う。
        // べき乗の行列でDivide and Conquerを使うため。
        let index_count = {
            let freq_index_count = (self.frequency_length / self.frequency_precision).ceil() as usize;
            let time_index_count = (self.time_length / self.time_precision).ceil() as usize;
            let max_index_count = freq_index_count.max(time_index_count);

            max_index_count.next_power_of_two()
        };
        assert!(index_count.is_power_of_two());

        // まず最後に求められる各Frequencyの情報をちゃんとした位置に入れるためのIndexルックアップテーブルを作る。
        // たとえば、index_count = 8のときに1番目のFrequency情報は4番目に入れるべきなど…
        let lookup_table = {
            // ビットリバーステクニックを使ってテーブルを作成。
            let mut results = vec![0];
            let mut addition_count = index_count >> 1;
            while addition_count > 0 {
                results.append(&mut results.iter().map(|v| v + addition_count).collect_vec());
                addition_count >>= 1;
            }

            results
        };

        // まず最後レベルの信号を計算する。index_count分作る。
        let final_signals = {
            let mut prev_signals: Vec<Complex<f64>> = vec![];
            prev_signals.reserve(index_count);

            // 無限に伸びる周期波形をつくるよりは、すでに与えられた波形をもっと細かく刻んでサンプルしたほうが安定そう。
            let time_precision = self.time_length / (index_count as f64);
            for i in 0..index_count {
                let amplitude = {
                    let cursor_time = self.time_start + (time_precision * (i as f64));
                    assert!(cursor_time <= time_end);

                    // アナログ波形に複素数の部分は存在しないので、Realパートだけ扱う。
                    let signal = container.uniform_sample_of_f64(cursor_time).unwrap().to_f64();
                    let window_factor = self.get_window_fn_factor(cursor_time);
                    signal * window_factor
                };

                prev_signals.push(Complex::<f64> {
                    real: amplitude,
                    imag: 0.0,
                });
            }

            //
            let mut next_signals: Vec<Complex<f64>> = vec![];
            next_signals.resize(index_count, <Complex<f64> as Default>::default());

            let level = (index_count as f64).log2().ceil() as usize;
            for lv_i in 0..level {
                let index_period = index_count >> lv_i;
                let half_index = index_period >> 1;

                for period_i in (0..index_count).step_by(index_period) {
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
        results.resize(index_count as usize, SineFrequency::default());

        let frequency_precision = self.frequency_length / (index_count as f64);
        for i in 0..(index_count as usize) {
            let target_i = lookup_table[i] as usize;

            let frequency = self.frequency_start + (frequency_precision * (target_i as f64));
            let sine_freq = SineFrequency::from_complex_f64(frequency, final_signals[i]);
            results[target_i] = sine_freq;
        }

        results
    }

    ///
    fn get_window_fn_factor(&self, time: f64) -> f64 {
        if let Some(window_fn) = self.window_function {
            window_fn.get_factor(self.time_start, self.time_length, time)
        } else {
            1f64
        }
    }
}

/// サイン波形の周波数の特性を表す。
#[derive(Default, Debug, Clone, Copy)]
pub struct SineFrequency {
    pub frequency: f64,
    pub amplitude: f32,
    pub phase: f32,
}

impl SineFrequency {
    pub fn from(frequency: f64, (freq_real, freq_imag): (f32, f32)) -> Self {
        Self {
            frequency,
            amplitude: (freq_real.powi(2) + freq_imag.powi(2)).sqrt(),
            phase: (freq_imag / freq_real).atan(),
        }
    }

    pub fn from_complex_f32(frequency: f32, complex: Complex<f32>) -> Self {
        Self {
            frequency: frequency as f64,
            amplitude: complex.absolute(),
            phase: complex.phase(),
        }
    }

    pub fn from_complex_f64(frequency: f64, complex: Complex<f64>) -> Self {
        Self {
            frequency,
            amplitude: complex.absolute() as f32,
            phase: complex.phase() as f32,
        }
    }
}
