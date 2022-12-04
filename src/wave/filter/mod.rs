use super::container::WaveContainer;
use crate::wave::PI2;
use itertools::Itertools;
mod dft;
mod fir;
mod iir;

#[derive(Debug, Clone)]
pub enum EEdgeFrequency {
    Constant(f64),
    ChangeBySample(fn(/* sample_i */ usize, /* samples_count */ usize) -> f64),
}

/// フィルタリングの機能
#[derive(Debug, Clone)]
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
        edge_frequency: EEdgeFrequency,
        /// クォリティファクタ
        quality_factor: f64,
    },
    /// IIR(Infinite Impulse Response)のHPF(High Pass Filter)
    IIRHighPass {
        /// エッジ周波数
        edge_frequency: EEdgeFrequency,
        /// クォリティファクタ
        quality_factor: f64,
    },
    IIRBandPass {
        /// 中心周波数
        center_frequency: EEdgeFrequency,
        /// クォリティファクタ
        quality_factor: f64,
    },
    IIRBandEliminate {
        /// 中心周波数
        center_frequency: EEdgeFrequency,
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
        /// オーバーラッピング機能を使うか？（Hann関数を基本使用する）
        use_overlap: bool,
    },
}

///
fn compute_fir_lpf_filters_count(delta: f64) -> usize {
    let mut filters_count = ((3.1 / delta).round() as isize) - 1;
    if (filters_count % 2) != 0 {
        filters_count += 1;
    }

    filters_count as usize
}

///
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

impl EFilter {
    ///
    pub fn apply_to_wave_container(&self, container: &WaveContainer) -> WaveContainer {
        // ここで書くには長いのでInternal構造体に移して処理を行う。
        match self {
            EFilter::FIRLowPass {
                edge_frequency,
                delta_frequency,
            } => fir::FIRLowPassInternal {
                edge_frequency: *edge_frequency,
                delta_frequency: *delta_frequency,
            }
            .apply(container),
            EFilter::IIRLowPass {
                edge_frequency,
                quality_factor,
            } => iir::LowPassInternal {
                edge_frequency: edge_frequency.clone(),
                quality_factor: *quality_factor,
            }
            .apply(container),
            EFilter::IIRHighPass {
                edge_frequency,
                quality_factor,
            } => iir::HighPassInternal {
                edge_frequency: edge_frequency.clone(),
                quality_factor: *quality_factor,
            }
            .apply(container),
            EFilter::IIRBandPass {
                center_frequency,
                quality_factor,
            } => iir::BandPassInternal {
                center_frequency: center_frequency.clone(),
                quality_factor: *quality_factor,
            }
            .apply(container),
            EFilter::IIRBandEliminate {
                center_frequency,
                quality_factor,
            } => iir::BandEliminateInternal {
                center_frequency: center_frequency.clone(),
                quality_factor: *quality_factor,
            }
            .apply(container),
            EFilter::DFTLowPass {
                edge_frequency,
                delta_frequency,
                max_input_samples_count,
                transform_compute_count,
                use_overlap,
            } => dft::DFTLowPassInternal {
                // ここで書くには長いのでInternal構造体に移して処理を行う。
                edge_frequency: *edge_frequency,
                delta_frequency: *delta_frequency,
                max_input_samples_count: *max_input_samples_count,
                transform_compute_count: *transform_compute_count,
                use_overlap: *use_overlap,
            }
            .apply(container),
        }
    }
}
