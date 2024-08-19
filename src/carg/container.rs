use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{container::WaveContainer, setting::WaveSound};

use super::v1;

/// @brief パーシングされたノードのコンテナ。
/// これだけで一連の処理ができる。
#[derive(Debug, Clone)]
pub enum ENodeContainer {
    None,
    V1 {
        input: v1::Input,
        setting: v1::Setting,
        output: v1::Output,
    },
}

impl ENodeContainer {
    pub fn process(&self) -> anyhow::Result<()> {
        match self {
            ENodeContainer::None => Ok(()),
            ENodeContainer::V1 { input, setting, output } => {
                let fmt_setting = setting.as_wave_format_setting();
                let sound_setting = input.into_sound_setting();

                let sound = WaveSound::from_setting(&fmt_setting, &sound_setting);
                let container = WaveContainer::from_wavesound(&sound).unwrap();

                {
                    let dest_file = fs::File::create(&output.file_name).expect("Could not create 500hz.wav.");
                    let mut writer = io::BufWriter::new(dest_file);
                    container.write(&mut writer);
                    writer.flush().expect("Failed to flush writer.")
                }

                Ok(())
            }
        }
    }
}
