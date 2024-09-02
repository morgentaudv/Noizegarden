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
            ProcessProcessorInput, TInputBufferOutputNone, TProcess,
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
    inputs: HashMap<usize, Vec<ProcessOutputBuffer>>,
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

impl OutputFileProcessData {
    fn update_state_stopped(&mut self, input: &ProcessProcessorInput) -> EProcessResult {
        // Childrenが全部送信完了したら処理が行える。
        // commonで初期Childrenの数を比較するだけでいいかも。
        if self.inputs.len() < self.common.child_count {
            return EProcessResult::Pending;
        }
        if input.children_states.iter().any(|v| *v != EProcessState::Finished) {
            return EProcessResult::Pending;
        }
        assert!(self.common.child_count > 0);

        // inputsのサンプルレートが同じかを確認する。
        let source_sample_rate = self.inputs.get(&0).unwrap().first().unwrap().setting.sample_rate;
        for (_, input) in self.inputs.iter().skip(1) {
            assert!(input.first().unwrap().setting.sample_rate == source_sample_rate);
        }

        // inputsを全部合わせる。
        let flattened_inputs = self
            .inputs
            .iter()
            .map(|(_, v)| v.iter().map(|w| w.buffer.clone()).flatten().collect_vec())
            .collect_vec();
        let range_inputs = self.inputs.iter().map(|(_, v)| v.first().unwrap().range).collect_vec();

        // ここで各bufferを組み合わせて、一つにしてから書き込む。
        let mut final_buffer_length = 0usize;
        let ref_vec = flattened_inputs
            .iter()
            .zip(range_inputs.iter())
            .map(|(buffer, range)| {
                let buffer_length = buffer.len();
                let start_index = (range.start * (source_sample_rate as f64)).floor() as usize;
                let exclusive_end_index = start_index + buffer_length;

                final_buffer_length = exclusive_end_index.max(final_buffer_length);

                (buffer, start_index)
            })
            .collect_vec();

        // 書き込み
        let mut new_buffer = vec![];
        new_buffer.resize(final_buffer_length, UniformedSample::default());
        for (ref_buffer, start_index) in ref_vec {
            for src_i in 0..ref_buffer.len() {
                let dest_i = start_index + src_i;
                new_buffer[dest_i] += ref_buffer[src_i];
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
                if !self.inputs.contains_key(&index) {
                    self.inputs.insert(index, vec![]);
                }

                self.inputs.get_mut(&index).unwrap().push(v);
            }
        }
    }
}

impl TProcess for OutputFileProcessData {
    /// データアイテムの処理が終わったか？
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn get_state(&self) -> EProcessState {
        self.common.state
    }

    fn try_process(&mut self, input: &ProcessProcessorInput) -> EProcessResult {
        match self.common.state {
            EProcessState::Stopped => self.update_state_stopped(input),
            EProcessState::Finished => {
                return EProcessResult::Finished;
            }
            _ => unreachable!("Unexpected branch"),
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
