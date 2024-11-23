use super::{container::WaveContainer, sample::UniformedSample};
use crate::wave::PI2;
use itertools::Itertools;
mod dft;
mod other;

#[derive(Debug, Clone, Copy)]
pub struct FilterCommonSetting {
    pub channel: u32,
    pub samples_per_second: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct FilterADSR {
    pub attack_sample_len: usize,
    pub decay_sample_len: usize,
    pub sustain_intensity: f64,
    pub release_sample_len: usize,
    pub gate_sample_len: usize,
    pub duration_sample_len: usize,
    /// 元となる周波数とADSRの計算によるIntensityを処理して最終的に使う周波数を返す。
    pub process_fn: fn(orig_freq: f64, adsr_intensity: f64) -> f64,
}

impl FilterADSR {
    pub fn compute(&self, sample_i: usize) -> f64 {
        assert!(self.gate_sample_len >= 1);
        assert!(self.attack_sample_len + self.decay_sample_len <= self.gate_sample_len);
        assert!(self.gate_sample_len <= self.duration_sample_len);

        match sample_i {
            x if (0..self.attack_sample_len).contains(&x) => match self.attack_sample_len {
                0 => 1.0,
                _ => 1.0 - (-5.0 * (sample_i as f64) / (self.attack_sample_len as f64)).exp(),
            },
            x if (self.attack_sample_len..self.gate_sample_len).contains(&x) => {
                let s = self.sustain_intensity;
                match self.decay_sample_len {
                    0 => s,
                    _ => {
                        let off = sample_i - self.attack_sample_len;
                        s + ((1.0 - s) * (-5.0 * (off as f64) / (self.decay_sample_len as f64)).exp())
                    }
                }
            }
            x if (self.gate_sample_len..self.duration_sample_len).contains(&x) => match self.release_sample_len {
                0 => 0.0,
                _ => {
                    let e = self.compute(self.gate_sample_len - 1);
                    let off = sample_i - self.gate_sample_len + 1;
                    e * (-5.0 * (off as f64) / (self.release_sample_len as f64)).exp()
                }
            },
            _ => 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum EEdgeFrequency {
    Constant(f64),
    ChangeBySample(fn(/* sample_i */ usize, /* samples_count */ usize, /* samples_per_sec */ usize) -> f64),
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
        /// 計算されたedge_frequencyにADSRの適用
        adsr: Option<FilterADSR>,
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

/// FIRのフィルターカウント計算。
pub fn compute_fir_filters_count(delta: f64) -> usize {
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
    pub fn apply_to_buffer(
        &self,
        common_setting: &FilterCommonSetting,
        buffer: &[UniformedSample],
    ) -> Vec<UniformedSample> {
        // ここで書くには長いのでInternal構造体に移して処理を行う。
        match self {
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
            .apply(common_setting, buffer),
            _ => unreachable!(),
        }
    }

    ///
    pub fn apply_to_wave_container(&self, container: &WaveContainer) -> WaveContainer {
        // ここで書くには長いのでInternal構造体に移して処理を行う。
        let common_setting = FilterCommonSetting {
            channel: container.channel(),
            samples_per_second: container.samples_per_second(),
        };
        let filtered_buffer = self.apply_to_buffer(&common_setting, container.uniformed_sample_buffer());

        WaveContainer::from_uniformed_sample_buffer(&container, filtered_buffer)
    }
}

pub enum ESourceFilter {
    /// [De-Emphasize](https://www.fon.hum.uva.nl/praat/manual/Sound__De-emphasize__in-place____.html)
    Deemphasizer {
        coefficient: f64,
    },
    PreEmphasizer {
        coefficient: f64,
    },
    /// LFO (Low Frequency Oscillator)を使ってVCAに振幅の時間エンベロープを適用する。
    AmplitudeTremolo {
        initial_scale: f64,
        periodical_scale_factor: f64,
        period_time_frequency: f64,
        source_samples_per_second: f64,
    },
    /// Apply ADSR(Attack - Decay - Sustain - Release) to amplitude.
    AmplitudeADSR {
        attack_sample_len: usize,
        decay_sample_len: usize,
        sustain_intensity: f64,
        release_sample_len: usize,
        gate_sample_len: usize,
        duration_sample_len: usize,
    },
}

impl ESourceFilter {
    pub fn apply_to_buffer(&self, buffer: &[UniformedSample]) -> Vec<UniformedSample> {
        match self {
            ESourceFilter::Deemphasizer { coefficient } => other::DeemphasizerInternal {
                coefficient: *coefficient,
            }
            .apply(buffer),
            ESourceFilter::PreEmphasizer { coefficient } => other::PreEmphasizerInternal {
                coefficient: *coefficient,
            }
            .apply(buffer),
            ESourceFilter::AmplitudeTremolo {
                initial_scale,
                periodical_scale_factor,
                period_time_frequency,
                source_samples_per_second,
            } => other::AmplitudeTremoloInternal {
                initial_scale: *initial_scale,
                periodical_scale_factor: *periodical_scale_factor,
                period_time_frequency: *period_time_frequency,
                source_samples_per_second: *source_samples_per_second,
            }
            .apply(buffer),
            ESourceFilter::AmplitudeADSR {
                attack_sample_len,
                decay_sample_len,
                sustain_intensity,
                release_sample_len,
                gate_sample_len,
                duration_sample_len,
            } => other::AmplitudeADSRInternal {
                attack_sample_len: *attack_sample_len,
                decay_sample_len: *decay_sample_len,
                sustain_intensity: *sustain_intensity,
                release_sample_len: *release_sample_len,
                gate_sample_len: *gate_sample_len,
                duration_sample_len: *duration_sample_len,
            }
            .apply(buffer),
        }
    }
}
