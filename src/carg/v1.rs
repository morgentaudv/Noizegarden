use serde::{Deserialize, Serialize};
use soundprog::wave::setting::{EFrequencyItem, WaveFormatSetting, WaveSoundSetting, WaveSoundSettingBuilder};

use crate::math::frequency::EFrequency;

/// 入力ノード
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Input {
    /// 正弦波で出力する
    SineWave {
        default_freq: EFrequency,
        length: f64,
        intensity: f64,
        start_time: Option<f64>,
    },
    /// ノコギリ波形で出力する
    Sawtooth {
        default_freq: EFrequency,
        length: f64,
        intensity: f64,
        start_time: Option<f64>,
    },
    /// 三角波形を出力する
    Triangle {
        default_freq: EFrequency,
        length: f64,
        intensity: f64,
        start_time: Option<f64>,
    },
    /// 矩形波を出力する
    Square {
        default_freq: EFrequency,
        length: f64,
        intensity: f64,
        start_time: Option<f64>,
    },
    /// ホワイトノイズを出力する
    WhiteNoise {
        length: f64,
        intensity: f64,
        start_time: Option<f64>,
    },
    /// ピンクノイズを出力する。
    PinkNoise {
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
            Input::Triangle {
                default_freq,
                length,
                intensity,
                start_time,
            } => InputSoundSetting {
                sound: WaveSoundSettingBuilder::default()
                    .frequency(EFrequencyItem::Triangle {
                        frequency: default_freq.to_frequency(),
                    })
                    .length_sec(*length as f32)
                    .intensity(*intensity)
                    .build()
                    .unwrap(),
                start_time: *start_time,
            },
            Input::Square {
                default_freq,
                length,
                intensity,
                start_time,
            } => InputSoundSetting {
                sound: WaveSoundSettingBuilder::default()
                    .frequency(EFrequencyItem::Square {
                        frequency: default_freq.to_frequency(),
                    })
                    .length_sec(*length as f32)
                    .intensity(*intensity)
                    .build()
                    .unwrap(),
                start_time: *start_time,
            },
            Input::WhiteNoise {
                length,
                intensity,
                start_time,
            } => InputSoundSetting {
                sound: WaveSoundSettingBuilder::default()
                    .frequency(EFrequencyItem::WhiteNoise)
                    .length_sec(*length as f32)
                    .intensity(*intensity)
                    .build()
                    .unwrap(),
                start_time: *start_time,
            },
            Input::PinkNoise {
                length,
                intensity,
                start_time,
            } => InputSoundSetting {
                sound: WaveSoundSettingBuilder::default()
                    .frequency(EFrequencyItem::PinkNoise)
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
