use serde::{Deserialize, Serialize};
use soundprog::wave::setting::{WaveFormatSetting, WaveSoundSetting, WaveSoundSettingBuilder};

use crate::math::frequency::EFrequency;

/// 入力ノード
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Input {
    SineWave {
        default_freq: EFrequency,
        length: f64,
        intensity: f64,
    },
}

impl Input {
    pub fn into_sound_setting(&self) -> WaveSoundSetting {
        match self {
            Input::SineWave {
                default_freq,
                length,
                intensity,
            } => WaveSoundSettingBuilder::default()
                .frequency(soundprog::wave::setting::EFrequencyItem::Constant {
                    frequency: default_freq.to_frequency(),
                })
                .length_sec(*length as f32)
                .intensity(*intensity)
                .build()
                .unwrap(),
        }
    }
}

/// @brief 設定ノード
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Setting {
    sample_rate: u64,
    bit_depth: String,
}

impl Setting {
    /// [`WaveFormatSetting`]に変換する。
    pub fn as_wave_format_setting(&self) -> WaveFormatSetting {
        WaveFormatSetting {
            samples_per_sec: self.sample_rate as u32,
            bits_per_sample: {
                assert!(self.bit_depth == "linear-16");
                soundprog::wave::setting::EBitsPerSample::Bits16
            },
        }
    }
}

/// @brief 出力ノード
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Output {
    pub file_name: String,
}
