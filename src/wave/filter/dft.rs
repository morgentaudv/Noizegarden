use itertools::Itertools;

use crate::wave::{
    analyze::{EAnalyzeMethod, ETransformMethod, FrequencyAnalyzer, FrequencyTransformer, SineFrequency},
    filter::{compute_fir_lpf_filters_count, compute_fir_lpf_response},
    sample::UniformedSample,
};

use super::FilterCommonSetting;

pub(super) struct DFTLowPassInternal {
    /// エッジ周波数
    pub(super) edge_frequency: f64,
    /// 遷移帯域幅の総周波数範囲
    pub(super) delta_frequency: f64,
    /// フレーム別に入力するサンプルの最大数
    pub(super) max_input_samples_count: usize,
    /// フーリエ変換を行う時のサンプル周期
    pub(super) transform_compute_count: usize,
    /// オーバーラッピング機能を使うか？（Hann関数を基本使用する）
    pub(super) use_overlap: bool,
}

impl DFTLowPassInternal {
    pub(super) fn apply(
        &self,
        common_setting: &FilterCommonSetting,
        read_buffer: &[UniformedSample],
    ) -> Vec<UniformedSample> {
        // ここではcontainerのチャンネルがMONO(1)だと仮定する。
        assert!(common_setting.channel == 1);

        // まずLPFでは標本周波数が1として前提して計算を行うので、edgeとdeltaも変換する。
        let samples_per_sec = common_setting.samples_per_second as f64;
        let edge = self.edge_frequency / samples_per_sec;
        let delta = self.delta_frequency / samples_per_sec;

        // フィルタ係数の数を計算する。
        // フィルタ係数の数は整数になるしかないし、またfilters_count+1が奇数じゃなきゃならない。
        // (Window Functionをちゃんと決めるため)
        let filters_count = compute_fir_lpf_filters_count(delta);
        let filter_responses = compute_fir_lpf_response(filters_count, edge);
        assert!(self.max_input_samples_count + filters_count <= self.transform_compute_count);

        // FFTで使うAnalzyer情報を記す。
        let input_analyzer = FrequencyAnalyzer {
            start_sample_index: 0,
            frequency_start: 1.0,
            samples_count: self.max_input_samples_count,
            window_function: None,
            analyze_method: EAnalyzeMethod::FFT,
        };
        let filter_analyzer = FrequencyAnalyzer {
            start_sample_index: 0,
            frequency_start: 1.0,
            samples_count: self.max_input_samples_count,
            window_function: None,
            analyze_method: EAnalyzeMethod::FFT,
        };
        let common_transformer = FrequencyTransformer {
            transform_method: ETransformMethod::IFFT,
        };

        // filter_responsesを用いて折りたたみを行う。
        let read_buffer_len = read_buffer.len();
        // DFTでできる最大のフレームを計算する。完全に割り切れなかった場合には残りは適当にする。
        let frames_to_compute = if self.use_overlap {
            // max_input_samples_countの半分おきにオーバーラップするので、
            // 最後のフレームは足りなくなるので１減らす。
            (read_buffer_len / (self.max_input_samples_count >> 1)) - 1
        } else {
            read_buffer_len / self.max_input_samples_count
        };
        let proceed_samples_count = if self.use_overlap {
            self.max_input_samples_count >> 1
        } else {
            self.max_input_samples_count
        };

        // B(m)リストを作る。
        let filter_buffer = {
            let mut buffer = filter_responses.iter().map(|v| UniformedSample::from_f64(*v)).collect_vec();
            buffer.resize(self.transform_compute_count, UniformedSample::default());
            buffer
        };

        let mut new_buffer = vec![];
        new_buffer.resize(read_buffer_len, UniformedSample::default());
        for frame_i in 0..frames_to_compute {
            let begin_sample_index = frame_i * proceed_samples_count;
            let end_sample_index = (begin_sample_index + self.max_input_samples_count).min(read_buffer_len);

            // X(n)リストを作る。
            let input_buffer = {
                // まずNカウント全部0で埋め尽くす。
                // ここではmax_input_samples_countを使わず、FFTのためのサンプルカウントでリストを作る。
                let mut buffer = vec![];
                buffer.resize(self.transform_compute_count, UniformedSample::default());

                // それから実際インプットのシグナル（実数）を最初から入れる。
                for load_i in begin_sample_index..end_sample_index {
                    let write_i = load_i - begin_sample_index;
                    buffer[write_i] = read_buffer[load_i];
                }
                buffer
            };

            // X(n)とB(m)を全部FFTをかける。
            let input_frequencies = input_analyzer
                .analyze_sample_buffer(&input_buffer)
                .expect("Failed to analyze input signal buffer.");
            let filter_frequencies = filter_analyzer
                .analyze_sample_buffer(&filter_buffer)
                .expect("Failed to analyze filter response buffer.");

            // Y(k) = B(k)X(k)なので計算してリスト化する。そしてY(k)をy(n)に逆変換する。
            let output_frequencies = filter_frequencies
                .iter()
                .zip(input_frequencies.iter())
                .map(|(filter, input)| {
                    assert!(filter.frequency == input.frequency);
                    let frequency = filter.frequency;
                    SineFrequency::from_complex_f64(frequency, filter.to_complex_f64() * input.to_complex_f64())
                })
                .collect_vec();
            let frame_result_buffer = common_transformer.transform_frequencies(&output_frequencies).unwrap();

            // 適切な位置に書き込む。
            for write_i in begin_sample_index..end_sample_index {
                let load_i = write_i - begin_sample_index;
                new_buffer[write_i] = frame_result_buffer[load_i];
            }
        }

        new_buffer
    }
}
