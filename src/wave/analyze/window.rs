use crate::wave::PI2;

/// 窓関数（Windowing Function）の種類の値を持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EWindowFunction {
    /// WindowFunctionを使わない。
    None,
    /// ハン窓関数を適用する。
    Hann,
}

impl EWindowFunction {
    /// 掛け算数値を計算する。もし範囲外なら、0だけを返す。
    pub fn get_factor(&self, length: f64, time: f64) -> f64 {
        // もし範囲外なら0を返す。
        if time < 0.0 || time > length {
            return 0f64;
        }

        let t = (time / length).clamp(0.0, 1.0);
        match self {
            EWindowFunction::Hann => {
                // 中央が一番高く、両端が0に収束する。
                (1f64 - (PI2 * t).cos()) * 0.5f64
            }
            EWindowFunction::None => 1.0,
        }
    }
}
