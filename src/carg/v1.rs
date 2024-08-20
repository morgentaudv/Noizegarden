use serde::{Deserialize, Serialize};
use soundprog::wave::setting::{WaveFormatSetting, WaveSoundSetting, WaveSoundSettingBuilder};

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

/// 入力周波数を表す
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum EFrequency {
    #[serde(rename = "constant")]
    Constant(f64),
    #[serde(rename = "a440")]
    A440ChromaticScale(EA440ChromaticScale),
}

impl EFrequency {
    pub const fn to_frequency(self) -> f64 {
        match self {
            EFrequency::Constant(freq) => freq,
            EFrequency::A440ChromaticScale(scale) => scale.to_frequency(),
        }
    }
}

/// 12微分音のA440スケールの音程
/// @link https://en.wikipedia.org/wiki/Chromatic_scale
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum EA440ChromaticScale {
    // 1 Octaves
    C1,
    C1S,
    D1,
    D1S,
    E1,
    F1,
    F1S,
    G1,
    G1S,
    A1,
    A1S,
    B1,
    // 2 Octaves
    C2,
    C2S,
    D2,
    D2S,
    E2,
    F2,
    F2S,
    G2,
    G2S,
    A2,
    A2S,
    B2,
    // 3 Octaves
    C3,
    C3S,
    D3,
    D3S,
    E3,
    F3,
    F3S,
    G3,
    G3S,
    A3,
    A3S,
    B3,
    // 4 Octaves
    C4,
    C4S,
    D4,
    D4S,
    E4,
    F4,
    F4S,
    G4,
    G4S,
    A4,
    A4S,
    B4,
    // 5 Octaves
    C5,
    C5S,
    D5,
    D5S,
    E5,
    F5,
    F5S,
    G5,
    G5S,
    A5,
    A5S,
    B5,
    // 6 Octaves
    C6,
    C6S,
    D6,
    D6S,
    E6,
    F6,
    F6S,
    G6,
    G6S,
    A6,
    A6S,
    B6,
    // 7 Octaves
    C7,
    C7S,
    D7,
    D7S,
    E7,
    F7,
    F7S,
    G7,
    G7S,
    A7,
    A7S,
    B7,
    // 8 Octaves
    C8,
}

impl EA440ChromaticScale {
    /// 周波数に変換する。逆はできない。
    pub const fn to_frequency(self) -> f64 {
        match self {
            EA440ChromaticScale::C1 => 32.703,
            EA440ChromaticScale::C1S => 34.648,
            EA440ChromaticScale::D1 => 36.708,
            EA440ChromaticScale::D1S => 38.891,
            EA440ChromaticScale::E1 => 41.203,
            EA440ChromaticScale::F1 => 43.654,
            EA440ChromaticScale::F1S => 46.249,
            EA440ChromaticScale::G1 => 48.999,
            EA440ChromaticScale::G1S => 51.913,
            EA440ChromaticScale::A1 => 55.000,
            EA440ChromaticScale::A1S => 58.270,
            EA440ChromaticScale::B1 => 61.735,
            EA440ChromaticScale::C2 => 65.406,
            EA440ChromaticScale::C2S => 69.296,
            EA440ChromaticScale::D2 => 73.416,
            EA440ChromaticScale::D2S => 77.782,
            EA440ChromaticScale::E2 => 82.407,
            EA440ChromaticScale::F2 => 87.307,
            EA440ChromaticScale::F2S => 92.499,
            EA440ChromaticScale::G2 => 97.999,
            EA440ChromaticScale::G2S => 103.826,
            EA440ChromaticScale::A2 => 110.000,
            EA440ChromaticScale::A2S => 116.541,
            EA440ChromaticScale::B2 => 123.471,
            EA440ChromaticScale::C3 => 130.813,
            EA440ChromaticScale::C3S => 138.591,
            EA440ChromaticScale::D3 => 146.832,
            EA440ChromaticScale::D3S => 155.563,
            EA440ChromaticScale::E3 => 164.814,
            EA440ChromaticScale::F3 => 174.614,
            EA440ChromaticScale::F3S => 184.997,
            EA440ChromaticScale::G3 => 195.998,
            EA440ChromaticScale::G3S => 207.652,
            EA440ChromaticScale::A3 => 220.000,
            EA440ChromaticScale::A3S => 233.082,
            EA440ChromaticScale::B3 => 246.942,
            EA440ChromaticScale::C4 => 261.626,
            EA440ChromaticScale::C4S => 277.183,
            EA440ChromaticScale::D4 => 293.665,
            EA440ChromaticScale::D4S => 311.127,
            EA440ChromaticScale::E4 => 329.628,
            EA440ChromaticScale::F4 => 349.228,
            EA440ChromaticScale::F4S => 369.994,
            EA440ChromaticScale::G4 => 391.995,
            EA440ChromaticScale::G4S => 415.305,
            EA440ChromaticScale::A4 => 440.000,
            EA440ChromaticScale::A4S => 466.164,
            EA440ChromaticScale::B4 => 493.883,
            EA440ChromaticScale::C5 => 523.251,
            EA440ChromaticScale::C5S => 554.365,
            EA440ChromaticScale::D5 => 587.330,
            EA440ChromaticScale::D5S => 622.254,
            EA440ChromaticScale::E5 => 659.255,
            EA440ChromaticScale::F5 => 698.456,
            EA440ChromaticScale::F5S => 793.989,
            EA440ChromaticScale::G5 => 783.991,
            EA440ChromaticScale::G5S => 830.609,
            EA440ChromaticScale::A5 => 880.000,
            EA440ChromaticScale::A5S => 932.328,
            EA440ChromaticScale::B5 => 987.767,
            EA440ChromaticScale::C6 => 1046.502,
            EA440ChromaticScale::C6S => 1108.731,
            EA440ChromaticScale::D6 => 1174.659,
            EA440ChromaticScale::D6S => 1244.508,
            EA440ChromaticScale::E6 => 1318.510,
            EA440ChromaticScale::F6 => 1396.913,
            EA440ChromaticScale::F6S => 1479.978,
            EA440ChromaticScale::G6 => 1567.982,
            EA440ChromaticScale::G6S => 1661.219,
            EA440ChromaticScale::A6 => 1760.000,
            EA440ChromaticScale::A6S => 1864.655,
            EA440ChromaticScale::B6 => 1975.533,
            EA440ChromaticScale::C7 => 2093.005,
            EA440ChromaticScale::C7S => 2217.461,
            EA440ChromaticScale::D7 => 2349.318,
            EA440ChromaticScale::D7S => 2489.016,
            EA440ChromaticScale::E7 => 2637.020,
            EA440ChromaticScale::F7 => 2793.826,
            EA440ChromaticScale::F7S => 2959.955,
            EA440ChromaticScale::G7 => 3135.963,
            EA440ChromaticScale::G7S => 3322.438,
            EA440ChromaticScale::A7 => 3520.000,
            EA440ChromaticScale::A7S => 3729.310,
            EA440ChromaticScale::B7 => 3951.066,
            EA440ChromaticScale::C8 => 4186.009,
        }
    }
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
