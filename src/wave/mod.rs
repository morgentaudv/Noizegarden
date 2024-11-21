use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

// WAVE (Waveform Audio File Format)
// https://so-zou.jp/software/tech/file/format/wav/
pub mod analyze;
pub mod complex;
pub mod container;
pub mod filter;
pub mod sample;
pub mod sine;
pub mod stretch;
pub mod time;

/// 2PIを示す。
pub const PI2: f64 = 2.0 * PI;

/// 秒を表す。
#[repr(transparent)]
pub struct Second(pub f64);

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EBitDepth {
    #[serde(rename = "linear_16")]
    Linear16,
}

impl EBitDepth {
    /// デシベルの範囲を表す。
    pub fn decibel_range(self) -> f64 {
        match self {
            EBitDepth::Linear16 => {
                ((1 << 16) as f64).log10() * 20.0
            }
        }
    }

    /// デシベルの表現地の最小値を返す。
    pub fn min_decibel(self) -> f64 {
        self.decibel_range() * -1.0
    }

    /// 入力の`decibel`を範囲に合わせてクランプする。
    pub fn clamp_decibel(self, decibel: f64) -> f64 {
        decibel.clamp(self.min_decibel(), 0.0)
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
