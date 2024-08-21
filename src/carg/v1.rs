use serde::{Deserialize, Serialize};
use soundprog::wave::setting::{EFrequencyItem, WaveFormatSetting, WaveSoundSetting, WaveSoundSettingBuilder};

use crate::math::frequency::EFrequency;

/// 入力ノード
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Input {
    SineWave {
        default_freq: EFrequency,
        length: f64,
        intensity: f64,
        start_time: Option<f64>,
    },
    Sawtooth {
        default_freq: EFrequency,
        length: f64,
        intensity: f64,
        start_time: Option<f64>,
    },
}

impl Input {
    pub fn into_sound_setting(&self) -> WaveSoundSetting {
        match self {
            Input::SineWave {
                default_freq,
                length,
                intensity,
                start_time,
            } => WaveSoundSettingBuilder::default()
                .frequency(EFrequencyItem::Constant {
                    frequency: default_freq.to_frequency(),
                })
                .length_sec(*length as f32)
                .intensity(*intensity)
                .build()
                .unwrap(),
            Input::Sawtooth {
                default_freq,
                length,
                intensity,
                start_time,
            } => WaveSoundSettingBuilder::default()
                .frequency(EFrequencyItem::Sawtooth {
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
    pub sample_rate: u64,
    pub bit_depth: String,
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
#[serde(tag = "type", content = "value")]
pub enum Output {
    #[serde(rename = "file")]
    File(EOutputFile),
}

/// ファイルとして出力するときのノード。
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum EOutputFile {
    #[serde(rename = "wav")]
    Wav {
        sample_rate: u64,
        bit_depth: String,
        file_name: String,
    },
}
