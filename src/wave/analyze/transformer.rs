use derive_builder::Builder;
use itertools::Itertools;

use crate::wave::{
    complex::Complex,
    sample::{self, UniformedSample},
    PI2,
};

use super::{sine_freq::SineFrequency, ETransformMethod};

/// Transformerのサンプル出力数の設定モード
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EExportSampleCountMode {
    #[default]
    Automatic,
    Fixed(usize),
}

#[derive(Debug, Default, Clone, Copy, Builder)]
#[builder(default)]
pub struct FrequencyTransformer {
    pub transform_method: ETransformMethod,
    pub sample_count_mode: EExportSampleCountMode,
}

impl FrequencyTransformer {
    pub fn transform_frequencies(&self, frequencies: &[SineFrequency]) -> Option<Vec<UniformedSample>> {
        // まずそれぞれの方法が使えるかを確認する。
        // たとえば、IFFTは周波数特性のサイズが2のべき乗じゃないとできない。
        if frequencies.is_empty() {
            return None;
        }

        match self.transform_method {
            ETransformMethod::IDFT => Some(self.transform_as_idft(frequencies)),
            ETransformMethod::IFFT => Some(self.transform_as_ifft(frequencies)),
        }
    }

    /// Inverse Discrete Fourier Transformを使って波形のサンプルリストに変換する。
    fn transform_as_idft(&self, frequencies: &[SineFrequency]) -> Vec<UniformedSample> {
        // まず0からtime_lengthまでのサンプルだけを収集する。
        // time_lengthの間のサンプル数を全部求めて
        //
        // ただ、DFTでの時間計算が [0, 1]範囲となっていたので、IDFTも同じくする？
        // とりあえずf64のサンプルに変換する。
        let samples_count = match self.sample_count_mode {
            EExportSampleCountMode::Automatic => frequencies.len(),
            EExportSampleCountMode::Fixed(v) => v,
        };
        if samples_count <= 0 {
            return vec![];
        }

        // 戻すこともO(N^2)
        let mut raw_samples = vec![];
        for time_i in 0..samples_count {
            let time_factor = (time_i as f64) / (samples_count as f64);

            // すべてのfrequency特性にイテレーションする。
            // a(k) * cos(2pik * time + phase)
            let summed: f64 = frequencies
                .iter()
                .map(|frequency| {
                    frequency.amplitude * ((PI2 * frequency.frequency * time_factor) + frequency.phase).cos()
                })
                .sum();

            // 1 / N (sigma)
            //let raw_sample = summed / analyzer.time_length;
            let raw_sample = summed / (samples_count as f64);
            raw_samples.push(raw_sample);
        }

        //for raw_samples in &raw_samples { println!("{:?}", raw_samples); }

        raw_samples
            .into_iter()
            .map(|raw_sample| UniformedSample::from_f64(raw_sample))
            .collect_vec()
    }

    /// Inverse Fast Fourier Transformを使って波形のサンプルリストに変換する。
    /// `frequencies`のサイズは必ず2のべき乗である必要がある。
    fn transform_as_ifft(&self, frequencies: &[SineFrequency]) -> Vec<UniformedSample> {
        assert!(frequencies.len().is_power_of_two());

        // FFTは2のべき乗数でしか入力出力できないので、
        // もし2べき乗じゃなければ後で出力された音幅リストをsamples_countに合わせる必要あり。
        let frequency_count = frequencies.len();
        let samples_count = match self.sample_count_mode {
            EExportSampleCountMode::Automatic => frequencies.len(),
            EExportSampleCountMode::Fixed(v) => v,
        };
        if samples_count <= 0 {
            return vec![];
        }

        // FFTから逆順で波形のAmplitudeを計算する。
        //
        // > まず最後に求められる各Frequencyの情報をちゃんとした位置に入れるためのIndexルックアップテーブルを作る。
        // > たとえば、index_count = 8のときに1番目のFrequency情報は4番目に入れるべきなど…
        //
        // FFTではそうだったが、IFFTではこの`lookup_table`からComplex情報を戻す。
        let lookup_table = {
            // ビットリバーステクニックを使ってテーブルを作成。
            let mut results = vec![0];
            let mut addition_count = frequency_count >> 1;
            while addition_count > 0 {
                results.append(&mut results.iter().map(|v| v + addition_count).collect_vec());
                addition_count >>= 1;
            }

            results
        };

        // ループしながら展開。
        let final_signals = {
            let lastlv_samples = {
                let mut lastlv_samples = vec![];
                lastlv_samples.resize(frequency_count, Complex::<f64>::default());

                for (write_i, search_i) in lookup_table.iter().enumerate() {
                    lastlv_samples[write_i] = frequencies[*search_i].to_complex_f64();
                }
                lastlv_samples
            };

            let mut prev_signals = lastlv_samples;
            let mut next_signals: Vec<Complex<f64>> = vec![];
            next_signals.resize(frequency_count, <Complex<f64> as Default>::default());

            // (level, 0]順で展開をする。
            let level = (frequency_count as f64).log2().ceil() as usize;
            for level_i in (0..level).rev() {
                let index_period = frequency_count >> level_i;
                let half_index = index_period >> 1;

                for period_i in (0..frequency_count).step_by(index_period) {
                    for local_i in 0..half_index {
                        // 計算過程
                        // prev[pli] = x + y
                        // prev[pri] = K(x - y) なので
                        // next[nli] = x = ((prev[pri] / K) + prev[pli]) / 2である。
                        // next[nri] = prev[pli] - xとなる。
                        let prev_lhs_i = period_i + local_i;
                        let prev_rhs_i = period_i + local_i + half_index;

                        let coefficient =
                            Complex::<f64>::from_exp(PI2 * (local_i as f64) / (index_period as f64)).conjugate();
                        let lhs_value = 0.5 * ((prev_signals[prev_rhs_i] / coefficient) + prev_signals[prev_lhs_i]);
                        let rhs_value = prev_signals[prev_lhs_i] - lhs_value;

                        let next_lhs_i = period_i + local_i;
                        let next_rhs_i = period_i + local_i + half_index;
                        next_signals[next_lhs_i] = lhs_value;
                        next_signals[next_rhs_i] = rhs_value;
                    }
                }

                // 次のレベルでprev→nextをするためにswapする。
                std::mem::swap(&mut prev_signals, &mut next_signals);
            }

            // Turncate
            if samples_count != frequency_count {
                prev_signals.truncate(samples_count);
            }
            prev_signals
        };

        // `final_signals`はまだComplexなので、しかし計算がちゃんとしていればimagはなくなると思う。
        // mapでrealだけを取得してUniformedSampleに変換する。
        final_signals
            .into_iter()
            .map(|signal| UniformedSample::from_f64(signal.real))
            .collect_vec()
    }
}
