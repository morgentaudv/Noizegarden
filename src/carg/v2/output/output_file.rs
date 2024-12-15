use std::{
    fs,
    io::{self, Write},
};

use crate::carg::v2::meta::input::{
    BufferMonoDynamicItem, BufferStereoDynamicItem, EInputContainerCategoryFlag, EProcessInputContainer
    ,
};
use crate::carg::v2::meta::output::EProcessOutputContainer;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::{ENode, SItemSPtr, Setting, TProcessItemPtr};

use crate::{
    carg::{
        v2::{ProcessControlItem, ProcessProcessorInput, TProcess},
    },
    wave::{
        container::WaveBuilder,
        stretch::pitch::{PitchShifterBufferSetting, PitchShifterBuilder},
    },
};
use crate::carg::v2::meta::system::TSystemCategory;
use crate::carg::v2::node::common::EProcessState;
use crate::carg::v2::output::EOutputFileFormat;
use crate::math::window::EWindowFunction;

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

impl TPinCategory for OutputFileProcessData {
    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの入力側のピンの名前を返す。
    fn get_input_pin_names() -> Vec<&'static str> {
        vec!["in"]
    }

    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの出力側のピンの名前を返す。
    fn get_output_pin_names() -> Vec<&'static str> {
        vec![]
    }

    /// 関係ノードに書いているピンのカテゴリ（複数可）を返す。
    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            "in" => Some(pin_category::BUFFER_MONO | pin_category::BUFFER_STEREO),
            _ => None,
        }
    }

    /// Inputピンのコンテナフラグ
    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            "in" => Some(input::container_category::OUTPUT_FILE),
            _ => None,
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

        let input = self.common.input_pins.get("in").unwrap().borrow().input.clone();
        match input {
            EProcessInputContainer::OutputFile(internal) => match internal {
                EOutputFileInput::Mono(v) => {
                    self.process_mono(&v);
                }
                EOutputFileInput::Stereo(v) => {
                    self.process_stereo(&v);
                }
            },
            _ => unreachable!("Unexpected input."),
        };

        // 状態変更。
        self.common.state = EProcessState::Finished;
    }

    fn process_mono(&mut self, v: &BufferMonoDynamicItem) {
        let source_sample_rate = v.setting.as_ref().unwrap().sample_rate as f64;

        let container = match self.format {
            EOutputFileFormat::WavLPCM16 { sample_rate } => {
                // もしsettingのsampling_rateがoutputのsampling_rateと違ったら、リサンプリングをしなきゃならない。
                let dest_sample_rate = sample_rate as f64;
                let processed_container = {
                    let pitch_rate = source_sample_rate / dest_sample_rate;
                    if pitch_rate == 1.0 {
                        v.buffer.clone()
                    } else {
                        PitchShifterBuilder::default()
                            .pitch_rate(pitch_rate)
                            .window_size(128)
                            .window_function(EWindowFunction::None)
                            .build()
                            .unwrap()
                            .process_with_buffer(&PitchShifterBufferSetting { buffer: &v.buffer })
                            .unwrap()
                    }
                };

                WaveBuilder {
                    samples_per_sec: sample_rate as u32,
                    bits_per_sample: 16,
                }
                .build_mono(processed_container)
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
    }

    fn process_stereo(&mut self, v: &BufferStereoDynamicItem) {
        let source_sample_rate = v.setting.as_ref().unwrap().sample_rate as f64;

        let container = match self.format {
            EOutputFileFormat::WavLPCM16 { sample_rate } => {
                // もしsettingのsampling_rateがoutputのsampling_rateと違ったら、リサンプリングをしなきゃならない。
                let dest_sample_rate = sample_rate as f64;
                let pitch_rate = source_sample_rate / dest_sample_rate;

                // Left Right 全部それぞれPitchShiftする。
                let (left, right) = {
                    if pitch_rate == 1.0 {
                        (v.ch_left.clone(), v.ch_right.clone())
                    } else {
                        let left = PitchShifterBuilder::default()
                            .pitch_rate(pitch_rate)
                            .window_size(128)
                            .window_function(EWindowFunction::None)
                            .build()
                            .unwrap()
                            .process_with_buffer(&PitchShifterBufferSetting { buffer: &v.ch_left })
                            .unwrap();
                        let right = PitchShifterBuilder::default()
                            .pitch_rate(pitch_rate)
                            .window_size(128)
                            .window_function(EWindowFunction::None)
                            .build()
                            .unwrap()
                            .process_with_buffer(&PitchShifterBufferSetting { buffer: &v.ch_right })
                            .unwrap();
                        (left, right)
                    }
                };

                WaveBuilder {
                    samples_per_sec: sample_rate as u32,
                    bits_per_sample: 16,
                }
                .build_stereo(left, right)
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
    }
}

impl TSystemCategory for OutputFileProcessData {}

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
// EOutputFileInput
// ----------------------------------------------------------------------------

/// [`EProcessInputContainer::OutputFile`]の内部コンテナ
#[derive(Debug, Clone)]
pub enum EOutputFileInput {
    Mono(BufferMonoDynamicItem),
    Stereo(BufferStereoDynamicItem),
}

impl EOutputFileInput {
    /// 今のセッティングで`output`が受け取れるか？
    pub fn can_support(&self, output: &EProcessOutputContainer) -> bool {
        match self {
            Self::Mono(_) => match output {
                EProcessOutputContainer::BufferMono(_) => true,
                _ => false,
            },
            Self::Stereo(_) => match output {
                EProcessOutputContainer::BufferStereo(_) => true,
                _ => false,
            },
        }
    }

    /// `output`からセッティングをリセットする。
    pub fn reset_with(&mut self, output: &EProcessOutputContainer) {
        if self.can_support(output) {
            return;
        }

        match output {
            EProcessOutputContainer::BufferMono(_) => {
                *self = Self::Mono(BufferMonoDynamicItem::new());
            }
            EProcessOutputContainer::BufferStereo(_) => {
                *self = Self::Stereo(BufferStereoDynamicItem::new());
            }
            _ => unreachable!("Unexpected branch"),
        }
    }

    /// 種類をかえずに中身だけをリセットする。
    pub fn reset(&mut self) {
        match self {
            Self::Mono(v) => {
                v.buffer.clear();
            }
            Self::Stereo(v) => {
                v.ch_left.clear();
                v.ch_right.clear();
            }
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
