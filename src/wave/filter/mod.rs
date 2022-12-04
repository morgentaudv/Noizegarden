use std::f64::consts::PI;

use super::container::WaveContainer;
use crate::wave::{
    analyze::{EAnalyzeMethod, ETransformMethod, FrequencyAnalyzer, FrequencyTransformer, SineFrequency},
    sample::UniformedSample,
    PI2,
};
use itertools::Itertools;

/// フィルタリングの機能
#[derive(Debug, Clone, Copy)]
pub enum EFilter {
    /// FIR(Finite Impulse Response)のLPF(Low Pass Filter)
    FIRLowPass {
        /// エッジ周波数
        edge_frequency: f64,
        /// 遷移帯域幅の総周波数範囲
        delta_frequency: f64,
    },
    /// IIR(Infinite Impulse Response)のLPF(Low Pass Filter)
    IIRLowPass {
        /// エッジ周波数
        edge_frequency: f64,
        /// クォリティファクタ
        quality_factor: f64,
    },
    /// DiscreteもしくはFastなFourier Transformを使ってLPFを行う。
    DFTLowPass {
        /// エッジ周波数
        edge_frequency: f64,
        /// 遷移帯域幅の総周波数範囲
        delta_frequency: f64,
        /// フレーム別に入力するサンプルの最大数
        max_input_samples_count: usize,
        /// フーリエ変換を行う時のサンプル周期
        transform_compute_count: usize,
    },
}

struct FIRLowPassInternal {
    /// エッジ周波数
    edge_frequency: f64,
    /// 遷移帯域幅の総周波数範囲
    delta_frequency: f64,
}

fn compute_fir_lpf_filters_count(delta: f64) -> usize {
    let mut filters_count = ((3.1 / delta).round() as isize) - 1;
    if (filters_count % 2) != 0 {
        filters_count += 1;
    }

    filters_count as usize
}

fn compute_fir_lpf_response(filters_count: usize, edge: f64) -> Vec<f64> {
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
            let input = PI2 * edge * (v as f64);
            let sinc = if input == 0.0 { 1.0 } else { input.sin() / input };

            2.0 * edge * sinc
        })
        .collect_vec();

    assert!(bs.len() == windows.len());
    for i in 0..windows.len() {
        bs[i] *= windows[i];
    }
    bs
}

impl FIRLowPassInternal {
    fn apply(&self, container: &WaveContainer) -> WaveContainer {
        // ここではcontainerのチャンネルがMONO(1)だと仮定する。
        assert!(container.channel() == 1);

        // まずLPFでは標本周波数が1として前提して計算を行うので、edgeとdeltaも変換する。
        let samples_per_sec = container.samples_per_second() as f64;
        let edge = self.edge_frequency / samples_per_sec;
        let delta = self.delta_frequency / samples_per_sec;

        // フィルタ係数の数を計算する。
        // フィルタ係数の数は整数になるしかないし、またfilters_count+1が奇数じゃなきゃならない。
        // (Window Functionをちゃんと決めるため)
        let filters_count = compute_fir_lpf_filters_count(delta);
        let filter_responses = compute_fir_lpf_response(filters_count, edge);

        // filter_responsesを用いて折りたたみを行う。
        let mut new_buffer = vec![];
        let orig_container = container.uniformed_sample_buffer();
        new_buffer.resize(orig_container.len(), UniformedSample::default());
        for sample_i in 0..new_buffer.len() {
            for fc_i in 0..=filters_count {
                if sample_i < fc_i {
                    break;
                }

                new_buffer[sample_i] += filter_responses[fc_i] * orig_container[sample_i - fc_i];
            }
        }

        WaveContainer::from_uniformed_sample_buffer(container, new_buffer)
    }
}

struct DFTLowPassInternal {
    /// エッジ周波数
    edge_frequency: f64,
    /// 遷移帯域幅の総周波数範囲
    delta_frequency: f64,
    /// フレーム別に入力するサンプルの最大数
    max_input_samples_count: usize,
    /// フーリエ変換を行う時のサンプル周期
    transform_compute_count: usize,
}

impl DFTLowPassInternal {
    fn apply(&self, container: &WaveContainer) -> WaveContainer {
        // ここではcontainerのチャンネルがMONO(1)だと仮定する。
        assert!(container.channel() == 1);

        // まずLPFでは標本周波数が1として前提して計算を行うので、edgeとdeltaも変換する。
        let samples_per_sec = container.samples_per_second() as f64;
        let edge = self.edge_frequency / samples_per_sec;
        let delta = self.delta_frequency / samples_per_sec;

        // フィルタ係数の数を計算する。
        // フィルタ係数の数は整数になるしかないし、またfilters_count+1が奇数じゃなきゃならない。
        // (Window Functionをちゃんと決めるため)
        let filters_count = compute_fir_lpf_filters_count(delta);
        let filter_responses = compute_fir_lpf_response(filters_count, edge);
        assert!(self.max_input_samples_count + filters_count <= self.transform_compute_count);

        // FFTで使うAnalzyer情報を記す。
        let common_analyzer = FrequencyAnalyzer {
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
        let orig_sample_buffer = container.uniformed_sample_buffer();
        let orig_sample_buffer_len = orig_sample_buffer.len();
        // DFTでできる最大のフレームを計算する。完全に割り切れなかった場合には残りは適当にする。
        let frames_to_compute = orig_sample_buffer_len / self.max_input_samples_count;
        // B(m)リストを作る。
        let filter_buffer = {
            let mut buffer = filter_responses.iter().map(|v| UniformedSample::from_f64(*v)).collect_vec();
            buffer.resize(self.transform_compute_count, UniformedSample::default());
            buffer
        };

        let mut new_buffer = vec![];
        new_buffer.resize(orig_sample_buffer_len, UniformedSample::default());
        for frame_i in 0..frames_to_compute {
            let begin_sample_index = frame_i * self.max_input_samples_count;
            let end_sample_index = ((frame_i + 1) * self.max_input_samples_count).min(orig_sample_buffer_len);

            // X(n)リストを作る。
            let input_buffer = {
                // まずNカウント全部0で埋め尽くす。
                let mut buffer = vec![];
                buffer.resize(self.transform_compute_count, UniformedSample::default());

                // それから実際インプットのシグナル（実数）を最初から入れる。
                for load_i in begin_sample_index..end_sample_index {
                    let write_i = load_i - begin_sample_index;
                    buffer[write_i] = orig_sample_buffer[load_i];
                }
                buffer
            };

            // X(n)とB(m)を全部FFTをかける。
            let input_frequencies = common_analyzer
                .analyze_sample_buffer(&input_buffer)
                .expect("Failed to analyze input signal buffer.");
            let filter_frequencies = common_analyzer
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
            for write_i in begin_sample_index..end_sample_index {
                let load_i = write_i - begin_sample_index;
                new_buffer[write_i] = frame_result_buffer[load_i];
            }
        }

        WaveContainer::from_uniformed_sample_buffer(container, new_buffer)
    }
}

struct IIRLowPassInternal {
    /// エッジ周波数
    edge_frequency: f64,
    /// クォリティファクタ
    quality_factor: f64,
}

impl IIRLowPassInternal {
    fn apply(&self, container: &WaveContainer) -> WaveContainer {
        // IIRはアナログの伝達関数を流用して（デジタルに適用できる形に変換）処理を行うけど
        // デジタル周波数はアナログ周波数に変換して使う。
        let samples_per_sec = container.samples_per_second() as f64;
        let analog_frequency = { 1.0 / (PI * 2.0) * (self.edge_frequency * PI / samples_per_sec).tan() };

        // そしてIIR-LPFの伝達関数の各乗算器の係数を求める。
        // まず共用の値を先に計算する。
        let (filter_bs, filter_as) = {
            let pi24a2 = 4.0 * PI.powi(2) * analog_frequency.powi(2);
            let pi2adivq = (2.0 * PI * analog_frequency) / self.quality_factor;
            let b1 = pi24a2 / (1.0 + pi2adivq + pi24a2);
            let b2 = 2.0 * b1;
            let b3 = b1;
            let a1 = (2.0 * pi24a2 - 2.0) / (1.0 + pi2adivq + pi24a2);
            let a2 = (1.0 - pi2adivq + pi24a2) / (1.0 + pi2adivq + pi24a2);

            ([b1, b2, b3], [1.0, a1, a2])
        };

        // 処理する。
        // IIR-LPFではB側で遅延機が２個、A側で遅延にが２個。
        let mut new_buffer = vec![];
        let orig_buffer = container.uniformed_sample_buffer();
        new_buffer.resize(orig_buffer.len(), UniformedSample::default());

        for i in 0..new_buffer.len() {
            for ji in 0..=2 {
                if i < ji {
                    break;
                }

                let bzxz = filter_bs[ji] * orig_buffer[i - ji];
                new_buffer[i] += bzxz;
            }
            for ji in 1..=2 {
                if i < ji {
                    break;
                }

                let azyz = filter_as[ji] * new_buffer[i - ji];
                new_buffer[i] -= azyz;
            }
        }

        WaveContainer::from_uniformed_sample_buffer(container, new_buffer)
    }
}

impl EFilter {
    ///
    pub fn apply_to_wave_container(&self, container: &WaveContainer) -> WaveContainer {
        match self {
            EFilter::FIRLowPass {
                edge_frequency,
                delta_frequency,
            } => FIRLowPassInternal {
                // ここで書くには長いのでInternal構造体に移して処理を行う。
                edge_frequency: *edge_frequency,
                delta_frequency: *delta_frequency,
            }
            .apply(container),
            EFilter::IIRLowPass {
                edge_frequency,
                quality_factor,
            } => IIRLowPassInternal {
                edge_frequency: *edge_frequency,
                quality_factor: *quality_factor,
            }
            .apply(container),
            EFilter::DFTLowPass {
                edge_frequency,
                delta_frequency,
                max_input_samples_count,
                transform_compute_count,
            } => DFTLowPassInternal {
                // ここで書くには長いのでInternal構造体に移して処理を行う。
                edge_frequency: *edge_frequency,
                delta_frequency: *delta_frequency,
                max_input_samples_count: *max_input_samples_count,
                transform_compute_count: *transform_compute_count,
            }
            .apply(container),
        }
    }
}
