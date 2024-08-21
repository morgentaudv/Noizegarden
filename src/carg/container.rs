use std::{
    fs,
    io::{self, Write},
};

use itertools::Itertools;
use soundprog::wave::{
    analyze::window::EWindowFunction,
    container::WaveBuilder,
    setting::WaveSound,
    stretch::pitch::{PitchShifterBufferSetting, PitchShifterBuilder},
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

fn process_v1(input: &Vec<v1::Input>, setting: &v1::Setting, output: &v1::Output) -> anyhow::Result<()> {
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

    match output {
        v1::Output::File { format, file_name } => {
            let container = match format {
                v1::EOutputFileFormat::WavLPCM16 { sample_rate } => {
                    // もしsettingのsampling_rateがoutputのsampling_rateと違ったら、
                    // リサンプリングをしなきゃならない。
                    let source_sample_rate = setting.sample_rate as f64;
                    let dest_sample_rate = *sample_rate as f64;

                    let processed_container = {
                        let pitch_rate = source_sample_rate / dest_sample_rate;
                        if pitch_rate == 1.0 {
                            buffer
                        } else {
                            PitchShifterBuilder::default()
                                .pitch_rate(pitch_rate)
                                .window_size(128)
                                .window_function(EWindowFunction::None)
                                .build()
                                .unwrap()
                                .process_with_buffer(&PitchShifterBufferSetting { buffer: &buffer })
                                .unwrap()
                        }
                    };

                    WaveBuilder {
                        samples_per_sec: *sample_rate as u32,
                        bits_per_sample: match fmt_setting.bits_per_sample {
                            soundprog::wave::setting::EBitsPerSample::Bits16 => 16,
                        },
                    }
                    .build_container(processed_container)
                    .unwrap()
                }
            };

            // 書き込み。
            {
                let dest_file = fs::File::create(&file_name).expect("Could not create 500hz.wav.");
                let mut writer = io::BufWriter::new(dest_file);
                container.write(&mut writer);
                writer.flush().expect("Failed to flush writer.")
            }
        }
    }

    Ok(())
}

impl ENodeContainer {
    pub fn process(&self) -> anyhow::Result<()> {
        match self {
            ENodeContainer::None => Ok(()),
            ENodeContainer::V1 { input, setting, output } => process_v1(input, setting, output),
        }
    }
}
