use clap::{Parser, ValueEnum};

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
}

/// [`EAppTestCommands::SineWave0`]のJson処理命令文
const JSON_SINE_WAVE_0: &str = r#"
{
    "mode": "test",
    "version": 1,
    "input": {
        "type": "SineWave",
        "default_freq": 440.0,
        "length": 3.0,
        "intensity": 1.0
    },
    "setting": {
        "sample_rate": 44100,
        "bit_depth": "linear-16"
    },
    "output": {
        "file_name": "test_sine_wave_0.wav"
    }
}
"#;

/// @brief テストコマンドごとのテスト処理のためのjson文字列を返す。
fn get_app_test_json(test_value: EAppTestCommands) -> String {
    match test_value {
        EAppTestCommands::SineWave0 => JSON_SINE_WAVE_0.to_owned(),
    }
}

/// @brief コマンド引数をパーシングする。
pub fn parse_command_arguments() -> anyhow::Result<()> {
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
            println!("{}, {}", parsed_info["mode"], parsed_info["version"]);

            // Input, Setting, Outputがちゃんとあるとみなして吐き出す。
            let input: v1::Input = serde_json::from_value(parsed_info["input"].clone())?;
            let setting: v1::Setting = serde_json::from_value(parsed_info["setting"].clone())?;
            let output: v1::Output = serde_json::from_value(parsed_info["output"].clone())?;
            println!("{:?}", input);
            println!("{:?}", setting);
            println!("{:?}", output);
        }
        // DO NOTHING
        None => (),
    }

    Ok(())
}
