/// 共通化したサンプルの振幅を表す。
///
/// 値の絶対値の0が一番低く、1が大きい。(0dBFS)
/// 値自体の範囲は [-1, 1]となる。
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
#[repr(transparent)]
pub struct UniformedSample(f64);

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

impl std::ops::Sub<Self> for UniformedSample {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl std::ops::SubAssign<Self> for UniformedSample {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl std::ops::Mul<UniformedSample> for f64 {
    type Output = UniformedSample;

    fn mul(self, rhs: UniformedSample) -> Self::Output {
        UniformedSample::from_f64(self * rhs.to_f64())
    }
}

impl std::ops::Mul<Self> for UniformedSample {
    type Output = UniformedSample;

    fn mul(self, rhs: Self) -> Self::Output {
        UniformedSample::from_f64(self.to_f64() * rhs.to_f64())
    }
}

impl UniformedSample {
    ///
    pub const MIN: UniformedSample = UniformedSample(0.0);

    /// 0から1まで（1は含まない）の範囲内の[`f64`]を変換する。
    ///
    /// ```
    /// # use soundprog::wave::sample::UniformedSample;
    /// let sample = UniformedSample::from_f64(0f64);
    /// # assert_eq!(sample, UniformedSample::MIN);
    /// ```
    pub fn from_f64(sample: f64) -> Self {
        Self(sample.clamp(-1.0, 1.0))
    }

    /// `[-32768, 32767)`までの16Bits振幅[`i16`]を変換する。
    pub fn from_16bits(sample: i16) -> Self {
        Self((sample as f64) / (i16::MAX as f64))
    }

    /// [`UniformedSample`]から量子化16ビットの[`i16`]に変換する。<br>
    /// [`i16`]で表現できない振幅値は削られて一番近い下の値に変換される。
    pub fn to_16bits(self) -> i16 {
        (self.0 * (i16::MAX as f64)).clamp(i16::MIN as f64, i16::MAX as f64) as i16
    }

    /// [`UniformedSample`]から量子化8ビットの[`u8`]に変換する。
    /// [`u8`]で表現できない振幅値はクランプされ一番近い値にクリッピングされる。
    ///
    /// ```
    /// # use soundprog::wave::sample::UniformedSample;
    /// assert_eq!(UniformedSample::from_f64(0f64).to_unsigned_8bits(), 127u8);
    /// assert_eq!(UniformedSample::from_f64(1f64).to_unsigned_8bits(), 254u8);
    /// assert_eq!(UniformedSample::from_f64(-1f64).to_unsigned_8bits(), 0u8);
    /// ```
    pub fn to_unsigned_8bits(self) -> u8 {
        ((self.0 * (i8::MAX as f64)) + (i8::MAX as f64))
            .clamp(0.0, u8::MAX as f64)
            .round() as u8
    }

    /// [`UniformedSample`]から量子化8ビットの[`i8`]に変換する。
    /// ただしu-lawの離散量子化アルゴリズムを使う。
    /// [`u8`]で表現できない振幅値はクランプされ一番近い値にクリッピングされる。
    pub fn to_ulaw_8bits(self) -> i8 {
        const LEVEL: [i16; 8] = [0x00FF, 0x01FF, 0x03FF, 0x07FF, 0x0FFF, 0x1FFF, 0x3FFF, 0x7FFF];

        let s16v = self.to_16bits();
        let mut abs16v = s16v.abs();
        if i16::MAX - abs16v >= 0x84 {
            abs16v += 0x84;
        } else {
            abs16v = i16::MAX;
        }

        let exponent = {
            let mut v = 9u8;
            for e in 0..8 {
                if abs16v <= LEVEL[e] {
                    v = e as u8;
                    break;
                }
            }
            v
        };

        // 必ずORで値を組み合わせする。
        let mantissa = ((abs16v >> (exponent + 3)) & 0x0F) as u8;
        let sign = if s16v.is_positive() { 0x00 } else { 0x80u8 as i8 };
        let i8v = ((exponent << 4) | mantissa) as i8 | sign;
        !i8v
    }

    /// [`f64`]に変換する。
    #[inline]
    pub fn to_f64(self) -> f64 {
        self.0
    }

    /// [`f64`]に変換するが、[-1, 1]範囲外の値はクランプされる。
    pub fn to_f64_clamped(self) -> f64 {
        self.0.clamp(-1.0, 1.0)
    }
}
