use std::{
    fs,
    io::{self, Write},
};

use itertools::Itertools;
use soundprog::wave::{
    container::{WaveBuilder, WaveContainer},
    setting::WaveSound,
};

use super::v1;

/// @brief パーシングされたノードのコンテナ。
/// これだけで一連の処理ができる。
#[derive(Debug)]
pub enum ENodeContainer {
    None,
    V1 {
        input: Vec<v1::Input>,
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

                let buffer = {
                    let buffers = input
                        .into_iter()
                        .map(|v| {
                            let sound_setting = v.into_sound_setting();
                            let sound = WaveSound::from_setting(&fmt_setting, &sound_setting);

                            let mut buffer = vec![];
                            for mut fragment in sound.sound_fragments {
                                buffer.append(&mut fragment.buffer);
                            }
                            buffer
                        })
                        .collect_vec();

                    let mut new_buffer = vec![];
                    for mut buffer in buffers {
                        new_buffer.append(&mut buffer);
                    }
                    new_buffer
                };

                let container = WaveBuilder {
                    samples_per_sec: fmt_setting.samples_per_sec,
                    bits_per_sample: match fmt_setting.bits_per_sample {
                        soundprog::wave::setting::EBitsPerSample::Bits16 => 16,
                    },
                }
                .build_container(buffer)
                .unwrap();

                match output {
                    v1::Output::File(data) => match data {
                        v1::EOutputFile::Wav {
                            sample_rate,
                            bit_depth,
                            file_name,
                        } => {
                            // もしsettingのsampling_rateがoutputのsampling_rateと違ったら、
                            // リサンプリングをしなきゃならない。
                            assert_eq!(setting.sample_rate, *sample_rate);
                            assert_eq!(setting.bit_depth, *bit_depth);

                            {
                                let dest_file = fs::File::create(&file_name).expect("Could not create 500hz.wav.");
                                let mut writer = io::BufWriter::new(dest_file);
                                container.write(&mut writer);
                                writer.flush().expect("Failed to flush writer.")
                            }
                        }
                    },
                }

                Ok(())
            }
        }
    }
}
