use std::{
    fs,
    io::{self, Write},
};

use crate::carg::v2::meta::input::EProcessInputContainer;
use crate::carg::v2::meta::ENodeSpecifier;
use crate::carg::v2::{ENode, SItemSPtr, Setting, TProcessItemPtr};
use crate::{
    carg::{
        v1::EOutputFileFormat,
        v2::{
            EProcessState, ProcessControlItem,
            ProcessProcessorInput, TProcess,
        },
    },
    wave::{
        analyze::window::EWindowFunction,
        container::WaveBuilder,
        stretch::pitch::{PitchShifterBufferSetting, PitchShifterBuilder},
    },
};

#[derive(Debug)]
pub struct OutputFileProcessData {
    common: ProcessControlItem,
    format: EOutputFileFormat,
    file_name: String,
}

impl OutputFileProcessData {
    pub fn create_from(node: &ENode, _setting: &Setting) -> TProcessItemPtr {
        match node {
            ENode::OutputFile { format, file_name } => {
                let item = Self::new(format.clone(), file_name.clone());
                SItemSPtr::new(item)
            }
            _ => unreachable!("Unexpected branch."),
        }
    }

    fn new(format: EOutputFileFormat, file_name: String) -> Self {
        Self {
            common: ProcessControlItem::new(ENodeSpecifier::OutputFile),
            format: format.clone(),
            file_name: file_name.clone(),
        }
    }
}

impl OutputFileProcessData {
    fn update_state(&mut self, input: &ProcessProcessorInput) {
        // Childrenが全部送信完了したら処理が行える。
        // commonで初期Childrenの数を比較するだけでいいかも。
        if !input.is_children_all_finished() {
            return;
        }

        let input = &self.common.input_pins.get("in").unwrap().borrow().input;
        let (buffer, source_sample_rate) = match input {
            EProcessInputContainer::WaveBuffersDynamic(v) => {
                (v.buffer.clone(), v.setting.as_ref().unwrap().sample_rate)
            }
            _ => unreachable!("Unexpected input."),
        };

        let container = match self.format {
            EOutputFileFormat::WavLPCM16 { sample_rate } => {
                // もしsettingのsampling_rateがoutputのsampling_rateと違ったら、
                // リサンプリングをしなきゃならない。
                let source_sample_rate = source_sample_rate as f64;
                let dest_sample_rate = sample_rate as f64;

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
    }
}

impl TProcess for OutputFileProcessData {
    /// データアイテムの処理が終わったか？
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn can_process(&self) -> bool {
        self.common.is_all_input_pins_update_notified()
    }

    fn get_common_ref(&self) -> &ProcessControlItem {
        &self.common
    }

    fn get_common_mut(&mut self) -> &mut ProcessControlItem {
        &mut self.common
    }

    fn try_process(&mut self, input: &ProcessProcessorInput) {
        self.common.elapsed_time = input.common.elapsed_time;
        self.common.process_input_pins();

        match self.common.state {
            EProcessState::Stopped | EProcessState::Playing => self.update_state(input),
            _ => (),
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
