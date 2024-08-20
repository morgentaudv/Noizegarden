use clap::{Parser, ValueEnum};
use container::ENodeContainer;

pub mod container;
pub mod v1;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CommandArgs {
    /// Application test option.
    #[arg(long, value_enum)]
    app_test: Option<EAppTestCommands>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, ValueEnum)]
enum EAppTestCommands {
    /// Output 440Hz sine wave at 44.1kHz, 16Bits LPCM for 3 Seconds.
    SineWave0,
    /// Output from C4 to C5 sine wave at 44.1kHz, 16Bits LPCM for 3 Seconds.
    SineWave1,
    /// Output A4 sawtooth wave with 50 order at 44.1kHz, 16Bits LPCM for 3 seconds.
    Sawtooth0,
}

/// [`EAppTestCommands::SineWave0`]のJson処理命令文
const JSON_SINE_WAVE_0: &str = r#"
{
    "mode": "test",
    "version": 1,
    "input": [
        {
            "type": "SineWave",
            "default_freq": {
                "type": "a440",
                "value": "A4"
            },
            "length": 3.0,
            "intensity": 1.0
        }
    ],
    "setting": {
        "sample_rate": 44100,
        "bit_depth": "linear-16"
    },
    "output": {
        "file_name": "test_sine_wave_0.wav"
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
        "file_name": "test_sine_wave_1.wav"
    }
}
"#;

/// [`EAppTestCommands::Sawtooth0`]のJson処理命令文
const JSON_SAWTOOTH_WAVE_0: &str = r#"
{
    "mode": "test",
    "version": 1,
    "input": [
        {
            "type": "Sawtooth",
            "default_freq": {
                "type": "a440",
                "value": "A4"
            },
            "length": 3.0,
            "intensity": 0.177
        }
    ],
    "setting": {
        "sample_rate": 44100,
        "bit_depth": "linear-16"
    },
    "output": {
        "file_name": "test_sawtooth_0.wav"
    }
}
"#;

/// @brief テストコマンドごとのテスト処理のためのjson文字列を返す。
fn get_app_test_json(test_value: EAppTestCommands) -> String {
    match test_value {
        EAppTestCommands::SineWave0 => JSON_SINE_WAVE_0.to_owned(),
        EAppTestCommands::SineWave1 => JSON_SINE_WAVE_1.to_owned(),
        EAppTestCommands::Sawtooth0 => JSON_SAWTOOTH_WAVE_0.to_owned(),
    }
}

/// @brief コマンド引数をパーシングする。
pub fn parse_command_arguments() -> anyhow::Result<ENodeContainer> {
    let cli = CommandArgs::parse();
    match cli.app_test {
        Some(test_value) => {
            // Parsing
            let json_str = get_app_test_json(test_value);
            let parsed_info: serde_json::Value = serde_json::from_str(&json_str)?;

            // チェック。今はassertで。
            {
                let parsed_mode = &parsed_info["mode"];
                assert!(parsed_mode.is_string() && parsed_mode.as_str().unwrap() == "test");
            }
            {
                let version = &parsed_info["version"];
                assert!(version.as_i64().unwrap() == 1);
            }

            // Input, Setting, Outputがちゃんとあるとみなして吐き出す。
            let input: Vec<v1::Input> = serde_json::from_value(parsed_info["input"].clone())?;
            let setting: v1::Setting = serde_json::from_value(parsed_info["setting"].clone())?;
            let output: v1::Output = serde_json::from_value(parsed_info["output"].clone())?;

            // まとめて出力。
            let container = ENodeContainer::V1 { input, setting, output };
            return Ok(container);
        }
        // DO NOTHING
        None => (),
    }

    Ok(ENodeContainer::None)
}