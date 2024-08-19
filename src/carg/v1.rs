use serde::{Deserialize, Serialize};

/// @brief 入力ノード
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Input {
    SineWave {
        default_freq: f64,
        length: f64,
        intensity: f64,
    },
}

/// @brief 設定ノード
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Setting {
    sample_rate: u64,
    bit_depth: String,
}

/// @brief 出力ノード
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Output {
    file_name: String,
}
