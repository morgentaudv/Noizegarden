use serde::{Deserialize, Serialize};
use soundprog::wave::{
    sample,
    setting::{EFrequencyItem, WaveFormatSetting, WaveSoundSetting, WaveSoundSettingBuilder},
};

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

#[derive(Debug)]
pub struct InputSoundSetting {
    pub sound: WaveSoundSetting,
    pub start_time: Option<f64>,
}

impl InputSoundSetting {
    pub fn explicit_buffer_start_index(&self, sample_rate: u64) -> Option<usize> {
        match self.start_time {
            Some(v) => Some((v * (sample_rate as f64)).floor() as usize),
            None => None,
        }
    }
}

impl Input {
    pub fn into_sound_setting(&self) -> InputSoundSetting {
        match self {
            Input::SineWave {
                default_freq,
                length,
                intensity,
                start_time,
            } => InputSoundSetting {
                sound: WaveSoundSettingBuilder::default()
                    .frequency(EFrequencyItem::Constant {
                        frequency: default_freq.to_frequency(),
                    })
                    .length_sec(*length as f32)
                    .intensity(*intensity)
                    .build()
                    .unwrap(),
                start_time: *start_time,
            },
            Input::Sawtooth {
                default_freq,
                length,
                intensity,
                start_time,
            } => InputSoundSetting {
                sound: WaveSoundSettingBuilder::default()
                    .frequency(EFrequencyItem::Sawtooth {
                        frequency: default_freq.to_frequency(),
                    })
                    .length_sec(*length as f32)
                    .intensity(*intensity)
                    .build()
                    .unwrap(),
                start_time: *start_time,
            },
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
    File {
        format: EOutputFileFormat,
        file_name: String,
    },
}

/// ファイルとして出力するときのノード。
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum EOutputFileFormat {
    #[serde(rename = "wav_lpcm16")]
    WavLPCM16 { sample_rate: u64 },
}
