use serde::{Deserialize, Serialize};
use crate::wave::PI2;

/// 窓関数（Windowing Function）の種類の値を持つ。
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum EWindowFunction {
    /// Rectangular Window
    #[serde(rename = "none")]
    None,
    /// [Hann Window](https://en.wikipedia.org/wiki/Window_function#Hann_and_Hamming_windows)
    #[serde(rename = "hann")]
    Hann,
    /// [Hann Window](https://en.wikipedia.org/wiki/Window_function#Hann_and_Hamming_windows)の
    /// Hamming Window部分を参考すること。
    #[serde(rename = "hamming")]
    Hamming,
    /// [Blackman Window](https://en.wikipedia.org/wiki/Window_function#Blackman_window)
    #[serde(rename = "blackman")]
    Blackman,
}

impl Default for EWindowFunction {
    fn default() -> Self {
        Self::None
    }
}

impl EWindowFunction {
    /// 掛け算数値を計算する。もし範囲外なら、0だけを返す。
    pub fn get_factor_time(&self, length: f64, time: f64) -> f64 {
        let t = (time / length).clamp(0.0, 1.0);
        match self {
            Self::None => 1.0,
            Self::Hann => {
                // もし範囲外なら0を返す。
                if time < 0.0 || time > length {
                    return 0f64;
                }

                // 中央が一番高く、両端が0に収束する。
                (1f64 - (PI2 * t).cos()) * 0.5f64
            }
            Self::Hamming => {
                // もし範囲外なら0を返す。
                if time < 0.0 || time > length {
                    return 0f64;
                }

                const A0: f64 = 0.53836;
                const A1: f64 = 1.0 - A0;

                // Hammingは両サイドが0ではない、が、最初のSidelobeをなくす。
                let c2pn_n = (PI2 * t).cos();
                A0 + (A1 * c2pn_n)
            }
            Self::Blackman => {
                const A0: f64 = 0.42;
                const A1: f64 = 0.5;
                const A2: f64 = 0.08;

                let c2pn_n = (PI2 * t).cos();
                let c4pn_n = (PI2 * 2.0 * t).cos();
                A0 - (A1 * c2pn_n) + (A2 * c4pn_n)
            }
        }
    }

    pub fn get_factor_samples(&self, sample_i: usize, sample_count: usize) -> f64 {
        self.get_factor_time(sample_count as f64, sample_i as f64)
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------

