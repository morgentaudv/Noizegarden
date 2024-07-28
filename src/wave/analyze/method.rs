/// @brief 変換モード
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EAnalyzeMethod {
    #[default]
    DFT,
    FFT,
}

/// @brief 逆変換モード
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ETransformMethod {
    #[default]
    IDFT,
    IFFT,
}
