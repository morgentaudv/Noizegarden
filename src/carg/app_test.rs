/// [`EAppTestCommands::SineWave0`]のJson処理命令文
const JSON_SINE_WAVE_0: &str = r#"
{
    "mode": "test",
    "version": 1,
    "input": [
        {
            "type": "SineWave",
            "default_freq": { "type": "a440", "value": "A4" },
            "length": 3.0,
            "intensity": 1.0
        }
    ],
    "setting": {
        "sample_rate": 44100,
        "bit_depth": "linear-16"
    },
    "output": {
        "type": "file",
        "value": {
            "format": {
                "type": "wav_lpcm16",
                "sample_rate": 44100
            },
            "file_name": "test_sine_wave_0.wav"
        }
    }
}
"#;

/// [`EAppTestCommands::SineWave1`]のJson処理命令文
const JSON_SINE_WAVE_1: &str = r#"
{
    "mode": "test",
    "version": 1,
    "input": [
        {
            "type": "SineWave",
            "default_freq": { "type": "a440", "value": "C4" },
            "length": 0.5,
            "intensity": 1.0
        },
        {
            "type": "SineWave",
            "default_freq": { "type": "a440", "value": "D4" },
            "length": 0.5,
            "intensity": 1.0
        },
        {
            "type": "SineWave",
            "default_freq": { "type": "a440", "value": "E4" },
            "length": 0.5,
            "intensity": 1.0
        },
        {
            "type": "SineWave",
            "default_freq": { "type": "a440", "value": "F4" },
            "length": 0.5,
            "intensity": 1.0
        },
        {
            "type": "SineWave",
            "default_freq": { "type": "a440", "value": "G4" },
            "length": 0.5,
            "intensity": 1.0
        },
        {
            "type": "SineWave",
            "default_freq": { "type": "a440", "value": "A4" },
            "length": 0.5,
            "intensity": 1.0
        },
        {
            "type": "SineWave",
            "default_freq": { "type": "a440", "value": "B4" },
            "length": 0.5,
            "intensity": 1.0
        },
        {
            "type": "SineWave",
            "default_freq": { "type": "a440", "value": "C5" },
            "length": 1.5,
            "intensity": 1.0
        }
    ],
    "setting": {
        "sample_rate": 44100,
        "bit_depth": "linear-16"
    },
    "output": {
        "type": "file",
        "value": {
            "format": {
                "type": "wav_lpcm16",
                "sample_rate": 44100
            },
            "file_name": "test_sine_wave_1.wav"
        }
    }
}
"#;

/// [`EAppTestCommands::SineWave2`]のJson処理命令文
const JSON_SINE_WAVE_2: &str = r#"
{
    "mode": "test",
    "version": 1,
    "input": [
        {
            "type": "SineWave",
            "default_freq": { "type": "a440", "value": "C4" },
            "length": 3.0,
            "intensity": 0.15,
            "start_time": 0.0
        },
        {
            "type": "SineWave",
            "default_freq": { "type": "a440", "value": "E4" },
            "length": 3.0,
            "intensity": 0.15,
            "start_time": 0.0
        },
        {
            "type": "SineWave",
            "default_freq": { "type": "a440", "value": "G4" },
            "length": 3.0,
            "intensity": 0.15,
            "start_time": 0.0
        }
    ],
    "setting": {
        "sample_rate": 44100,
        "bit_depth": "linear-16"
    },
    "output": {
        "type": "file",
        "value": {
            "format": {
                "type": "wav_lpcm16",
                "sample_rate": 44100
            },
            "file_name": "test_sine_wave_2.wav"
        }
    }
}
"#;

/// [`EAppTestCommands::Sawtooth`]のJson処理命令文
const JSON_SAWTOOTH_WAVE: &str = r#"
{
    "mode": "test",
    "version": 1,
    "input": [
        {
            "type": "Sawtooth",
            "default_freq": { "type": "a440", "value": "A4" },
            "length": 3.0,
            "intensity": 0.177
        }
    ],
    "setting": {
        "sample_rate": 44100,
        "bit_depth": "linear-16"
    },
    "output": {
        "type": "file",
        "value": {
            "format": {
                "type": "wav_lpcm16",
                "sample_rate": 22050
            },
            "file_name": "test_sawtooth_0_22050.wav"
        }
    }
}
"#;

/// [`EAppTestCommands::Triangle`]のJson処理命令文
const JSON_TRIANGLE_WAVE: &str = r#"
{
    "mode": "test",
    "version": 1,
    "input": [
        {
            "type": "Triangle",
            "default_freq": { "type": "a440", "value": "A4" },
            "length": 3.0,
            "intensity": 0.177
        }
    ],
    "setting": {
        "sample_rate": 44100,
        "bit_depth": "linear-16"
    },
    "output": {
        "type": "file",
        "value": {
            "format": {
                "type": "wav_lpcm16",
                "sample_rate": 44100
            },
            "file_name": "test_triangle.wav"
        }
    }
}
"#;

/// [`EAppTestCommands::Square`]のJson処理命令文
const JSON_SQUARE_WAVE: &str = r#"
{
    "mode": "test",
    "version": 1,
    "input": [
        {
            "type": "Square",
            "default_freq": { "type": "a440", "value": "A4" },
            "length": 3.0,
            "intensity": 0.177
        }
    ],
    "setting": {
        "sample_rate": 44100,
        "bit_depth": "linear-16"
    },
    "output": {
        "type": "file",
        "value": {
            "format": {
                "type": "wav_lpcm16",
                "sample_rate": 44100
            },
            "file_name": "test_square.wav"
        }
    }
}
"#;

/// [`EAppTestCommands::WhiteNoise`]のJson処理命令文
const JSON_WHITENOISE_WAVE: &str = r#"
{
    "mode": "test",
    "version": 1,
    "input": [ { "type": "WhiteNoise", "length": 3.0, "intensity": 0.177 } ],
    "setting": {
        "sample_rate": 44100,
        "bit_depth": "linear-16"
    },
    "output": {
        "type": "file",
        "value": {
            "format": { "type": "wav_lpcm16", "sample_rate": 44100 },
            "file_name": "test_whitenoise.wav"
        }
    }
}
"#;

/// [`EAppTestCommands::PinkNoise`]のJson処理命令文
const JSON_PINKNOISE_WAVE: &str = r#"
{
    "mode": "test",
    "version": 1,
    "input": [ { "type": "PinkNoise", "length": 3.0, "intensity": 0.5 } ],
    "setting": {
        "sample_rate": 44100,
        "bit_depth": "linear-16"
    },
    "output": {
        "type": "file",
        "value": {
            "format": { "type": "wav_lpcm16", "sample_rate": 44100 },
            "file_name": "test_pinknoise.wav"
        }
    }
}
"#;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, clap::ValueEnum)]
pub enum EAppTestCommands {
    /// Output 440Hz sine wave at 44.1kHz, 16Bits LPCM for 3 Seconds.
    SineWave0,
    /// Output from C4 to C5 sine wave at 44.1kHz, 16Bits LPCM for 3 Seconds.
    SineWave1,
    /// Output C4, E4, G4 (CMajor) sine wave together at 44.1kHz, 16Bits LPCM for 3 seconds.
    SineWave2,
    /// Output A4 sawtooth wave at 44.1kHz, 16Bits LPCM for 3 seconds.
    Sawtooth,
    /// Output A4 triangle wave at 44.1kHz, 16Bits LPCM for 3 seconds.
    Triangle,
    /// Output A4 square wave at 44.1kHz, 16Bits LPCM for 3 seconds.
    Square,
    /// Output whitenoise wave at 44.1kHz, 16Bits LPCM for 3 seconds.
    WhiteNoise,
    /// Output pink noise wave at 44.1kHz, 16Bits LPCM for 3 seconds.
    PinkNoise,
}

/// @brief テストコマンドごとのテスト処理のためのjson文字列を返す。
pub fn get_app_test_json(test_value: EAppTestCommands) -> String {
    match test_value {
        EAppTestCommands::SineWave0 => JSON_SINE_WAVE_0.to_owned(),
        EAppTestCommands::SineWave1 => JSON_SINE_WAVE_1.to_owned(),
        EAppTestCommands::SineWave2 => JSON_SINE_WAVE_2.to_owned(),
        EAppTestCommands::Sawtooth => JSON_SAWTOOTH_WAVE.to_owned(),
        EAppTestCommands::Triangle => JSON_TRIANGLE_WAVE.to_owned(),
        EAppTestCommands::Square => JSON_SQUARE_WAVE.to_owned(),
        EAppTestCommands::WhiteNoise => JSON_WHITENOISE_WAVE.to_owned(),
        EAppTestCommands::PinkNoise => JSON_PINKNOISE_WAVE.to_owned(),
    }
}
