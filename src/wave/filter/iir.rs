use crate::wave::{sample::UniformedSample};
use super::{EEdgeFrequency, FilterADSR, FilterCommonSetting};

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
    /// フィルターを適用する。
    pub(super) fn apply(
        &self,
        _common_setting: &FilterCommonSetting,
        _read_buffer: &[UniformedSample],
    ) -> Vec<UniformedSample> {
        vec![]
    }
}

#[deprecated]
pub(super) struct HighPassInternal {
    /// エッジ周波数
    pub(super) edge_frequency: EEdgeFrequency,
    /// クォリティファクタ
    pub(super) quality_factor: f64,
}

impl HighPassInternal {
    pub(super) fn apply(
        &self,
        _common_setting: &FilterCommonSetting,
        _read_buffer: &[UniformedSample],
    ) -> Vec<UniformedSample> {
        vec![]
    }
}

#[deprecated]
pub(super) struct BandPassInternal {
    /// 中心周波数
    pub(super) center_frequency: EEdgeFrequency,
    /// クォリティファクタ
    pub(super) quality_factor: f64,
}

impl BandPassInternal {
    pub(super) fn apply(
        &self,
        _common_setting: &FilterCommonSetting,
        _read_buffer: &[UniformedSample],
    ) -> Vec<UniformedSample> {
        vec![]
    }
}

#[deprecated]
pub(super) struct BandEliminateInternal {
    /// 中心周波数
    pub(super) center_frequency: EEdgeFrequency,
    /// クォリティファクタ
    pub(super) quality_factor: f64,
}

impl BandEliminateInternal {
    pub(super) fn apply(
        &self,
        common_setting: &FilterCommonSetting,
        read_buffer: &[UniformedSample],
    ) -> Vec<UniformedSample> {
        vec![]
    }
}
