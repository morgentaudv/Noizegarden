use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
};

use itertools::Itertools;

use crate::{
    carg::{
        v1::EOutputFileFormat,
        v2::{
            EProcessOutput, EProcessResult, EProcessState, ProcessControlItem, ProcessOutputBuffer,
            TInputBufferOutputNone, TProcess,
        },
    },
    wave::{
        analyze::window::EWindowFunction,
        container::WaveBuilder,
        sample::UniformedSample,
        stretch::pitch::{PitchShifterBufferSetting, PitchShifterBuilder},
    },
};

#[derive(Debug)]
pub struct OutputFileProcessData {
    common: ProcessControlItem,
    format: EOutputFileFormat,
    file_name: String,
    inputs: HashMap<usize, ProcessOutputBuffer>,
}

impl OutputFileProcessData {
    pub fn new(format: EOutputFileFormat, file_name: String) -> Self {
        Self {
            common: ProcessControlItem::new(),
            format: format.clone(),
            file_name: file_name.clone(),
            inputs: HashMap::new(),
        }
    }
}

impl TInputBufferOutputNone for OutputFileProcessData {
    /// 自分のタイムスタンプを返す。
    fn get_timestamp(&self) -> i64 {
        self.common.process_timestamp
    }

    fn set_child_count(&mut self, count: usize) {
        self.common.child_count = count;
    }

    fn update_input(&mut self, index: usize, output: EProcessOutput) {
        match output {
            EProcessOutput::None => unimplemented!("Unexpected branch."),
            EProcessOutput::Buffer(v) => {
                self.inputs.insert(index, v);
            }
        }
    }
}

impl TProcess for OutputFileProcessData {
    /// データアイテムの処理が終わったか？
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn try_process(&mut self, input: &crate::carg::v2::ProcessInput) -> EProcessResult {
        // Childrenが全部送信完了したら処理が行える。
        // commonで初期Childrenの数を比較するだけでいいかも。
        if self.inputs.len() < self.common.child_count {
            return EProcessResult::Pending;
        }
        assert!(self.common.child_count > 0);

        // inputsのサンプルレートが同じかを確認する。
        let source_sample_rate = self.inputs.get(&0).unwrap().setting.sample_rate;
        for (_, input) in self.inputs.iter().skip(1) {
            assert!(input.setting.sample_rate == source_sample_rate);
        }

        // ここで各bufferを組み合わせて、一つにしてから書き込む。
        let mut final_buffer_length = 0usize;
        let ref_vec = self
            .inputs
            .iter()
            .map(|(_, info)| {
                let buffer_length = info.buffer.len();
                let start_index = (info.range.start * (info.setting.sample_rate as f64)).floor() as usize;
                let exclusive_end_index = start_index + buffer_length;

                final_buffer_length = exclusive_end_index.max(final_buffer_length);

                (info, start_index)
            })
            .collect_vec();

        // 書き込み
        let mut new_buffer = vec![];
        new_buffer.resize(final_buffer_length, UniformedSample::default());
        for (ref_buffer, start_index) in ref_vec {
            for src_i in 0..ref_buffer.buffer.len() {
                let dest_i = start_index + src_i;
                new_buffer[dest_i] += ref_buffer.buffer[src_i];
            }
        }

        let container = match self.format {
            EOutputFileFormat::WavLPCM16 { sample_rate } => {
                // もしsettingのsampling_rateがoutputのsampling_rateと違ったら、
                // リサンプリングをしなきゃならない。
                let source_sample_rate = source_sample_rate as f64;
                let dest_sample_rate = sample_rate as f64;

                let processed_container = {
                    let pitch_rate = source_sample_rate / dest_sample_rate;
                    if pitch_rate == 1.0 {
                        new_buffer
                    } else {
                        PitchShifterBuilder::default()
                            .pitch_rate(pitch_rate)
                            .window_size(128)
                            .window_function(EWindowFunction::None)
                            .build()
                            .unwrap()
                            .process_with_buffer(&PitchShifterBufferSetting { buffer: &new_buffer })
                            .unwrap()
                    }
                };

                WaveBuilder {
                    samples_per_sec: sample_rate as u32,
                    bits_per_sample: 16,
                }
                .build_container(processed_container)
                .unwrap()
            }
        };

        // 書き込み。
        {
            let dest_file = fs::File::create(&self.file_name).expect("Could not create 500hz.wav.");
            let mut writer = io::BufWriter::new(dest_file);
            container.write(&mut writer);
            writer.flush().expect("Failed to flush writer.")
        }

        // 状態変更。
        self.common.state = EProcessState::Finished;
        self.common.process_timestamp += 1;
        return EProcessResult::Finished;
    }
}
