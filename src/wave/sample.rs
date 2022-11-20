use std::ops::{Shl, Shr};

/// 共通化したサンプルの振幅を表す。
/// 0が一番低く、[`std::u32::MAX`]が一番大きい。
///
/// 32ビットまでの各量子化ビットへの変換で精度を保つために[`u32`]に統一する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct UniformedSample(u32);

impl UniformedSample {
    ///
    pub const MIN: UniformedSample = UniformedSample(0);

    /// 0から1までの範囲内の[`f64`]を変換する。
    ///
    /// ```
    /// # use soundprog::wave::sample::UniformedSample;
    /// let sample = UniformedSample::from_f64(-1f64);
    /// # assert_eq!(sample, UniformedSample::MIN);
    /// ```
    pub fn from_f64(sample: f64) -> Self {
        assert!(sample >= -1.0 && sample <= 1.0);

        let scaled_sample = ((sample + 1.0) / 2.0) * (u32::MAX as f64);
        Self(scaled_sample.floor() as u32)
    }

    /// `[-32768, 32767)`までの16Bits振幅[`i16`]を変換する。
    ///
    /// ```
    /// ```
    pub fn from_16bits(sample: i16) -> Self {
        let added = (sample as i32 + 32768i32) as u32;
        assert!(added >= 0 && added <= (u16::MAX as u32));

        let result = added.shl(16);
        Self(result)
    }

    /// [`UniformedSample`]から量子化16ビットの[`i16`]に変換する。<br>
    /// [`i16`]で表現できない振幅値は削られて一番近い下の値に変換される。
    pub fn to_16bits(self) -> i16 {
        let shifted = self.0.shr(16) as i32;
        (shifted - 32768) as i16
    }
}
