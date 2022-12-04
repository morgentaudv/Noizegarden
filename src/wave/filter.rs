use std::f64::consts::PI;

use itertools::Itertools;

use crate::wave::{sample::UniformedSample, PI2};

use super::container::WaveContainer;

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
    },
}

struct FIRLowPassInternal {
    /// エッジ周波数
    edge_frequency: f64,
    /// 遷移帯域幅の総周波数範囲
    delta_frequency: f64,
}

impl FIRLowPassInternal {
    fn apply(&self, container: &WaveContainer) -> WaveContainer {
        // まずLPFでは標本周波数が1として前提して計算を行うので、edgeとdeltaも変換する。
        let samples_per_sec = container.samples_per_second() as f64;
        let edge = self.edge_frequency / samples_per_sec;
        let delta = self.delta_frequency / samples_per_sec;

        // フィルタ係数の数を計算する。
        // フィルタ係数の数は整数になるしかないし、またJ+1が奇数じゃなきゃならない。
        // (Window Functionをちゃんと決めるため)
        let mut j = ((3.1 / delta).round() as isize) - 1;
        if (j % 2) != 0 {
            j += 1;
        }

        // ここではcontainerのチャンネルがMONO(1)だと仮定する。
        assert!(container.channel() == 1);
        let filter_bs = {
            // -J/2からJ/2までにEWindowFunction(Hann)の値リストを求める。
            let windows = (0..=j)
                .map(|v| {
                    let sine = PI2 * ((v as f64) + 0.5) / ((j + 1) as f64);
                    (1.0 - sine) * 0.5
                })
                .collect_vec();

            // フィルタ係数の週はす特性bを計算する。
            let mut bs = (((j >> 1) * -1)..=(j >> 1))
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
        };

        // bsを用いて折りたたみを行う。
        let mut new_buffer = vec![];
        let orig_container = container.uniformed_sample_buffer();
        new_buffer.resize(orig_container.len(), UniformedSample::default());
        for i in 0..new_buffer.len() {
            for ji in 0..=(j as usize) {
                if i < ji {
                    break;
                }
                new_buffer[i] += filter_bs[ji] * orig_container[i - ji];
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
}

impl DFTLowPassInternal {
    fn apply(&self, container: &WaveContainer) -> WaveContainer {
        // まずLPFでは標本周波数が1として前提して計算を行うので、edgeとdeltaも変換する。
        let samples_per_sec = container.samples_per_second() as f64;
        let edge = self.edge_frequency / samples_per_sec;
        let delta = self.delta_frequency / samples_per_sec;

        // フィルタ係数の数を計算する。
        // フィルタ係数の数は整数になるしかないし、またJ+1が奇数じゃなきゃならない。
        // (Window Functionをちゃんと決めるため)
        let mut j = ((3.1 / delta).round() as isize) - 1;
        if (j % 2) != 0 {
            j += 1;
        }

        // ここではcontainerのチャンネルがMONO(1)だと仮定する。
        assert!(container.channel() == 1);
        let filter_bs = {
            // -J/2からJ/2までにEWindowFunction(Hann)の値リストを求める。
            let windows = (0..=j)
                .map(|v| {
                    let sine = PI2 * ((v as f64) + 0.5) / ((j + 1) as f64);
                    (1.0 - sine) * 0.5
                })
                .collect_vec();

            // フィルタ係数の週はす特性bを計算する。
            let mut bs = (((j >> 1) * -1)..=(j >> 1))
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
        };

        // bsを用いて折りたたみを行う。
        let mut new_buffer = vec![];
        let orig_container = container.uniformed_sample_buffer();
        new_buffer.resize(orig_container.len(), UniformedSample::default());
        for i in 0..new_buffer.len() {
            for ji in 0..=(j as usize) {
                if i < ji {
                    break;
                }
                new_buffer[i] += filter_bs[ji] * orig_container[i - ji];
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
            } => DFTLowPassInternal {
                // ここで書くには長いのでInternal構造体に移して処理を行う。
                edge_frequency: *edge_frequency,
                delta_frequency: *delta_frequency,
            }
            .apply(container),
        }
    }
}
