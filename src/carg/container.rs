use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
};

use crate::wave::{
    analyze::window::EWindowFunction,
    container::WaveBuilder,
    sample::UniformedSample,
    sine::setting::{EBitsPerSample, WaveSound},
    stretch::pitch::{PitchShifterBufferSetting, PitchShifterBuilder},
};
use itertools::Itertools;
use crate::carg::v2::meta;
use super::{
    v1,
    v2::{self},
};

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
    V2 {
        setting: v2::Setting,
        nodes: HashMap<String, v2::ENode>,
        relations: Vec<meta::relation::Relation>,
    },
}

impl ENodeContainer {
    pub fn process(&self) -> anyhow::Result<()> {
        match self {
            ENodeContainer::None => Ok(()),
            ENodeContainer::V1 { input, setting, output } => process_v1(input, setting, output),
            ENodeContainer::V2 {
                setting,
                nodes,
                relations,
            } => v2::process_v2(setting, nodes.clone(), relations),
        }
    }
}

struct UniformedSampleBufferItem {
    buffer: Vec<UniformedSample>,
    start_index: usize,
    length: usize,
}

fn process_v1(input: &Vec<v1::Input>, setting: &v1::Setting, output: &v1::Output) -> anyhow::Result<()> {
    let fmt_setting = setting.as_wave_format_setting();

    let buffer = {
        let sample_rate = fmt_setting.samples_per_sec as u64;

        // Inputそれぞれをバッファー化する。
        let mut fallback_buffer_start_index = 0usize;
        let buffers = input
            .into_iter()
            .map(|v| {
                let sound_setting = v.into_sound_setting();
                let sound = WaveSound::from_setting(&fmt_setting, &sound_setting.sound);

                let mut buffer = vec![];
                for fragment in sound.sound_fragments {
                    buffer.extend(&fragment.buffer);
                }

                let buffer_length = buffer.len();
                let start_index = match sound_setting.explicit_buffer_start_index(sample_rate) {
                    Some(i) => i,
                    None => fallback_buffer_start_index,
                };
                let buffer_next_start_index = start_index + buffer_length;
                fallback_buffer_start_index = buffer_next_start_index.max(fallback_buffer_start_index);

                UniformedSampleBufferItem {
                    buffer,
                    start_index,
                    length: buffer_length,
                }
            })
            .collect_vec();

        // 合わせる。
        let buffer_length = buffers.iter().map(|v| v.start_index + v.length).max().unwrap();
        let mut new_buffer = vec![];
        new_buffer.resize_with(buffer_length, Default::default);

        for buffer in buffers {
            for src_i in 0..buffer.length {
                let dest_i = buffer.start_index + src_i;
                new_buffer[dest_i] += buffer.buffer[src_i];
            }
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
                            EBitsPerSample::Bits16 => 16,
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
