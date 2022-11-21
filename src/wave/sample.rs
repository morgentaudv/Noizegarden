use std::ops::{Shl, Shr};

/// 共通化したサンプルの振幅を表す。
/// 0が一番低く、[`std::u32::MAX`]が一番大きい。
///
/// 32ビットまでの各量子化ビットへの変換で精度を保つために[`u32`]に統一する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct UniformedSample(i32);

impl std::ops::Add<Self> for UniformedSample {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign<Self> for UniformedSample {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl UniformedSample {
    ///
    pub const MIN: UniformedSample = UniformedSample(0);

    /// 0から1まで（1は含まない）の範囲内の[`f64`]を変換する。
    ///
    /// ```
    /// # use soundprog::wave::sample::UniformedSample;
    /// let sample = UniformedSample::from_f64(0f64);
    /// # assert_eq!(sample, UniformedSample::MIN);
    /// ```
    pub fn from_f64(sample: f64) -> Self {
        assert!(sample >= -1.0 && sample <= 1.0);

        let scaled_sample = sample * (i32::MAX as f64);
        Self(scaled_sample.floor() as i32)
    }

    /// `[-32768, 32767)`までの16Bits振幅[`i16`]を変換する。
    ///
    /// ```
    /// ```
    pub fn from_16bits(sample: i16) -> Self {
        Self((sample as i32).shl(16))
    }

    /// [`UniformedSample`]から量子化16ビットの[`i16`]に変換する。<br>
    /// [`i16`]で表現できない振幅値は削られて一番近い下の値に変換される。
    pub fn to_16bits(self) -> i16 {
        let shifted = self.0.shr(16) as i32;
        shifted as i16
    }
}
