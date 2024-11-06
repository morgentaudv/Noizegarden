use std::f64::consts::PI;

use crate::wave::{sample::UniformedSample, PI2};

use super::{EEdgeFrequency, FilterADSR, FilterCommonSetting};

fn compute_sample(
    write_sample_i: usize,
    write_feedback_buffer: &mut [UniformedSample],
    original_buffer: &[UniformedSample],
    filter_as: &[f64],
    filter_bs: &[f64],
) {
    debug_assert!(filter_as.len() == 3);
    debug_assert!(filter_bs.len() == 3);

    for ji in 0..=2 {
        if write_sample_i < ji {
            break;
        }

        let bzxz = filter_bs[ji] * original_buffer[write_sample_i - ji];
        write_feedback_buffer[write_sample_i] += bzxz;
    }
    for ji in 1..=2 {
        if write_sample_i < ji {
            break;
        }

        let azyz = filter_as[ji] * write_feedback_buffer[write_sample_i - ji];
        write_feedback_buffer[write_sample_i] -= azyz;
    }
}

#[deprecated]
pub(super) struct LowPassInternal {
    /// エッジ周波数
    pub(super) edge_frequency: EEdgeFrequency,
    /// クォリティファクタ
    pub(super) quality_factor: f64,
    /// 元となる周波数とADSRの計算によるIntensityを処理して最終的に使う周波数を返す。
    pub(super) adsr: Option<FilterADSR>,
}

impl LowPassInternal {
    /// に使う遅延機フィルターの伝達関数の特性を計算する。
    fn compute_filter_asbs(edge_frequency: f64, samples_per_sec: f64, quality_factor: f64) -> ([f64; 3], [f64; 3]) {
        let analog_frequency = { 1.0 / PI2 * (edge_frequency * PI / samples_per_sec).tan() };
        let pi24a2 = 4.0 * PI.powi(2) * analog_frequency.powi(2);
        let pi2adivq = (PI2 * analog_frequency) / quality_factor;
        let b1 = pi24a2 / (1.0 + pi2adivq + pi24a2);
        let b2 = 2.0 * b1;
        let b3 = b1;
        let a1 = (2.0 * pi24a2 - 2.0) / (1.0 + pi2adivq + pi24a2);
        let a2 = (1.0 - pi2adivq + pi24a2) / (1.0 + pi2adivq + pi24a2);

        ([1.0, a1, a2], [b1, b2, b3])
    }

    /// フィルターを適用する。
    pub(super) fn apply(
        &self,
        common_setting: &FilterCommonSetting,
        read_buffer: &[UniformedSample],
    ) -> Vec<UniformedSample> {
        // はアナログの伝達関数を流用して（デジタルに適用できる形に変換）処理を行うけど
        // デジタル周波数はアナログ周波数に変換して使う。
        let samples_per_sec = common_setting.samples_per_second as f64;

        // 処理する。
        // -LPFではB側で遅延機が２個、A側で遅延にが２個。
        let mut new_buffer = vec![];
        new_buffer.resize(read_buffer.len(), UniformedSample::default());

        let total_sample_count = new_buffer.len();
        for sample_i in 0..total_sample_count {
            // IIRの演算のための係数を計算する。
            // もしADSRがついてあれば、ADSRを反映して計算する。
            let (filter_as, filter_bs) = {
                let mut freq = match self.edge_frequency {
                    EEdgeFrequency::Constant(freq) => freq,
                    EEdgeFrequency::ChangeBySample(compute_fn) => {
                        compute_fn(sample_i, total_sample_count, common_setting.samples_per_second as usize)
                    }
                };

                // ADSRがあれば、使う。
                if let Some(adsr) = self.adsr.as_ref() {
                    let intensity = adsr.compute(sample_i);
                    freq = (adsr.process_fn)(freq, intensity);
                }

                Self::compute_filter_asbs(freq, samples_per_sec, self.quality_factor)
            };

            compute_sample(sample_i, &mut new_buffer, read_buffer, &filter_as, &filter_bs);
        }

        new_buffer
    }
}

pub(super) struct HighPassInternal {
    /// エッジ周波数
    pub(super) edge_frequency: EEdgeFrequency,
    /// クォリティファクタ
    pub(super) quality_factor: f64,
}

impl HighPassInternal {
    /// に使う遅延機フィルターの伝達関数の特性を計算する。
    fn compute_filter_asbs(edge_frequency: f64, samples_per_sec: f64, quality_factor: f64) -> ([f64; 3], [f64; 3]) {
        let analog_frequency = { 1.0 / PI2 * (edge_frequency * PI / samples_per_sec).tan() };
        // 4pi^2f_c^2
        let pi24a2 = 4.0 * PI.powi(2) * analog_frequency.powi(2);
        // 2pif_c / Q
        let pi2adivq = (PI2 * analog_frequency) / quality_factor;

        let b1 = 1.0 / (1.0 + pi2adivq + pi24a2);
        let b2 = -2.0 * b1;
        let b3 = b1;
        let a1 = (2.0 * pi24a2 - 2.0) * b1;
        let a2 = (1.0 - pi2adivq + pi24a2) * b1;

        ([1.0, a1, a2], [b1, b2, b3])
    }

    pub(super) fn apply(
        &self,
        common_setting: &FilterCommonSetting,
        read_buffer: &[UniformedSample],
    ) -> Vec<UniformedSample> {
        // はアナログの伝達関数を流用して（デジタルに適用できる形に変換）処理を行うけど
        // デジタル周波数はアナログ周波数に変換して使う。
        let samples_per_sec = common_setting.samples_per_second as f64;

        // もしEdgeFrequencyがサンプルによって動的に変わるのではなければ、HPFの伝達関数の各乗算器の係数を求める。
        // まず共用の値を先に計算する。
        let constant_filter_asbs = match self.edge_frequency {
            EEdgeFrequency::Constant(freq) => {
                Some(Self::compute_filter_asbs(freq, samples_per_sec, self.quality_factor))
            }
            EEdgeFrequency::ChangeBySample(_) => None,
        };

        // 処理する。
        // HPFではB側で遅延機が２個、A側で遅延にが２個。
        let mut new_buffer = vec![];
        new_buffer.resize(read_buffer.len(), UniformedSample::default());

        let total_sample_count = new_buffer.len();
        for i in 0..total_sample_count {
            let (filter_as, filter_bs) = if let EEdgeFrequency::ChangeBySample(compute_func) = &self.edge_frequency {
                let edge_frequency = compute_func(i, total_sample_count, common_setting.samples_per_second as usize);
                Self::compute_filter_asbs(edge_frequency, samples_per_sec, self.quality_factor)
            } else {
                constant_filter_asbs.clone().unwrap()
            };

            compute_sample(i, &mut new_buffer, read_buffer, &filter_as, &filter_bs);
        }

        new_buffer
    }
}

pub(super) struct BandPassInternal {
    /// 中心周波数
    pub(super) center_frequency: EEdgeFrequency,
    /// クォリティファクタ
    pub(super) quality_factor: f64,
}

impl BandPassInternal {
    /// IIRに使う遅延機フィルターの伝達関数の特性を計算する。
    fn compute_filter_asbs(center_frequency: f64, samples_per_sec: f64, quality_factor: f64) -> ([f64; 3], [f64; 3]) {
        let analog_frequency = { 1.0 / PI2 * (center_frequency * PI / samples_per_sec).tan() };
        // 4pi^2f_c^2
        let pi24a2 = 4.0 * PI.powi(2) * analog_frequency.powi(2);
        // 2pif_c / Q
        let pi2adivq = (PI2 * analog_frequency) / quality_factor;
        let div_base = 1.0 + pi2adivq + pi24a2;

        let b1 = pi2adivq / div_base;
        let b2 = 0.0;
        let b3 = b1 * -1.0;
        let a1 = (2.0 * pi24a2 - 2.0) / div_base;
        let a2 = (1.0 - pi2adivq + pi24a2) / div_base;

        ([1.0, a1, a2], [b1, b2, b3])
    }

    pub(super) fn apply(
        &self,
        common_setting: &FilterCommonSetting,
        read_buffer: &[UniformedSample],
    ) -> Vec<UniformedSample> {
        // はアナログの伝達関数を流用して（デジタルに適用できる形に変換）処理を行うけど
        // デジタル周波数はアナログ周波数に変換して使う。
        let samples_per_sec = common_setting.samples_per_second as f64;

        // もしCenterFrequencyがサンプルによって動的に変わるのではなければ、BPFの伝達関数の各乗算器の係数を求める。
        // まず共用の値を先に計算する。
        let constant_filter_asbs = match self.center_frequency {
            EEdgeFrequency::Constant(freq) => {
                Some(Self::compute_filter_asbs(freq, samples_per_sec, self.quality_factor))
            }
            EEdgeFrequency::ChangeBySample(_) => None,
        };

        // 処理する。
        // BPFではB側で遅延機が２個、A側で遅延にが２個。
        let mut new_buffer = vec![];
        new_buffer.resize(read_buffer.len(), UniformedSample::default());

        let total_sample_count = new_buffer.len();
        for i in 0..total_sample_count {
            let (filter_as, filter_bs) = if let EEdgeFrequency::ChangeBySample(compute_func) = &self.center_frequency {
                let edge_frequency = compute_func(i, total_sample_count, common_setting.samples_per_second as usize);
                Self::compute_filter_asbs(edge_frequency, samples_per_sec, self.quality_factor)
            } else {
                constant_filter_asbs.clone().unwrap()
            };

            compute_sample(i, &mut new_buffer, read_buffer, &filter_as, &filter_bs);
        }

        new_buffer
    }
}

pub(super) struct BandEliminateInternal {
    /// 中心周波数
    pub(super) center_frequency: EEdgeFrequency,
    /// クォリティファクタ
    pub(super) quality_factor: f64,
}

impl BandEliminateInternal {
    /// IIRに使う遅延機フィルターの伝達関数の特性を計算する。
    fn compute_filter_asbs(center_frequency: f64, samples_per_sec: f64, quality_factor: f64) -> ([f64; 3], [f64; 3]) {
        let analog_frequency = { 1.0 / PI2 * (center_frequency * PI / samples_per_sec).tan() };
        // 4pi^2f_c^2
        let pi24a2 = 4.0 * PI.powi(2) * analog_frequency.powi(2);
        // 2pif_c / Q
        let pi2adivq = (PI2 * analog_frequency) / quality_factor;
        let div_base = 1.0 + pi2adivq + pi24a2;

        let b1 = (pi24a2 + 1.0) / div_base;
        let b2 = (2.0 * pi24a2 - 2.0) / div_base;
        let b3 = b1;
        let a1 = b2;
        let a2 = (1.0 - pi2adivq + pi24a2) / div_base;

        ([1.0, a1, a2], [b1, b2, b3])
    }

    pub(super) fn apply(
        &self,
        common_setting: &FilterCommonSetting,
        read_buffer: &[UniformedSample],
    ) -> Vec<UniformedSample> {
        // IIRはアナログの伝達関数を流用して（デジタルに適用できる形に変換）処理を行うけど
        // デジタル周波数はアナログ周波数に変換して使う。
        let samples_per_sec = common_setting.samples_per_second as f64;

        // もしCenterFrequencyがサンプルによって動的に変わるのではなければ、BEFの伝達関数の各乗算器の係数を求める。
        // まず共用の値を先に計算する。
        let constant_filter_asbs = match self.center_frequency {
            EEdgeFrequency::Constant(freq) => {
                Some(Self::compute_filter_asbs(freq, samples_per_sec, self.quality_factor))
            }
            EEdgeFrequency::ChangeBySample(_) => None,
        };

        // 処理する。
        // BEFではB側で遅延機が２個、A側で遅延にが２個。
        let mut new_buffer = vec![];
        new_buffer.resize(read_buffer.len(), UniformedSample::default());

        let total_sample_count = new_buffer.len();
        for i in 0..total_sample_count {
            let (filter_as, filter_bs) = if let EEdgeFrequency::ChangeBySample(compute_func) = &self.center_frequency {
                let edge_frequency = compute_func(i, total_sample_count, common_setting.samples_per_second as usize);
                Self::compute_filter_asbs(edge_frequency, samples_per_sec, self.quality_factor)
            } else {
                constant_filter_asbs.clone().unwrap()
            };

            compute_sample(i, &mut new_buffer, read_buffer, &filter_as, &filter_bs);
        }

        new_buffer
    }
}
