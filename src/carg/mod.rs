use std::{fs, io};

use app_test::EAppTestCommands;
use clap::{Parser, ValueEnum};
use container::ENodeContainer;

pub mod app_test;
pub mod container;
pub mod v1;
pub mod v2;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CommandArgs {
    /// Application test option.
    #[arg(long, value_enum)]
    app_test: Option<EAppTestCommands>,
    /// Raad setting json file as an input.
    #[arg(long, short)]
    input_file: Option<std::path::PathBuf>,
}

const TEST_JSON_STRING: &str = r#"
{
    "version": 2,
    "setting": {
        "sample_rate": 44100
    },
    "node": {
        "input": {
            "type": "emitter-pinknoise",
            "intensity": 0.5,
            "range": {
                "start": 0.0,
                "length": 3.0
            }
        },
        "output": {
            "type": "output-file",
            "format": {
                "type": "wav_lpcm16",
                "sample_rate": 44100
            },
            "file_name": "test_pinknoise.wav"
        }
    },
    "relation": [
        {
            "input": [
                "input"
            ],
            "output": "output"
        }
    ]
}
"#;

impl CommandArgs {
    ///
    fn try_parse_info(&self) -> anyhow::Result<serde_json::Value> {
        match self.app_test {
            Some(test_value) => {
                // Parsing
                let json_str = app_test::get_app_test_json(test_value);
                let info: serde_json::Value = serde_json::from_str(&json_str)?;
                return Ok(info);
            }
            // DO NOTHING
            None => (),
        }

        // Inputがあれば、パスがあるかを確認し読み取って処理する。
        match &self.input_file {
            Some(path) => {
                assert!(path.is_file());
                assert!(fs::exists(&path).is_ok());

                let opened_file = fs::File::open(path.as_path()).expect("Failed to open file.");
                let reader = io::BufReader::new(opened_file);
                let info: serde_json::Value = serde_json::from_reader(reader)?;
                return Ok(info);
            }
            None => (),
        }

        Err(anyhow::anyhow!("Failed to parse"))
    }

    fn try_test_parse_info() -> anyhow::Result<serde_json::Value> {
        let info: serde_json::Value = serde_json::from_str(TEST_JSON_STRING)?;
        Ok(info)
    }
}

/// @brief コマンド引数をパーシングする。
pub fn parse_command_arguments() -> anyhow::Result<ENodeContainer> {
    let cli = CommandArgs::parse();
    let parsed_info = cli.try_parse_info()?;

    // チェック。
    let version = parsed_info["version"].as_i64().expect("version should be interger.");
    match version {
        1 => {
            return v1::parse_v1(&parsed_info);
        }
        2 => {
            return v2::parse_v2(&parsed_info);
        }
        _ => (),
    }

    Ok(ENodeContainer::None)
}
