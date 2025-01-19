use crate::carg::v2::meta::input::{
    BufferMonoDynamicItem, BufferStereoDynamicItem, EInputContainerCategoryFlag, EProcessInputContainer,
};
use crate::carg::v2::meta::output::EProcessOutputContainer;
use crate::carg::v2::meta::system::{system_category, ESystemCategoryFlag, InitializeSystemAccessor, TSystemCategory};
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::node::common::{EProcessState, ProcessControlItemSetting};
use crate::carg::v2::output::EOutputFileFormat;
use crate::carg::v2::{ENode, ProcessItemCreateSetting, SItemSPtr, TProcessItem, TProcessItemPtr};
use crate::file::EFileAccessSetting;
use crate::math::window::EWindowFunction;
use crate::wave::sample::UniformedSample;
use crate::{
    carg::v2::{ProcessControlItem, ProcessProcessorInput, TProcess},
    wave::{
        container::WaveBuilder,
        stretch::pitch::{PitchShifterBufferSetting, PitchShifterBuilder},
    },
};
use chrono::Local;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaOutputFileInfo {
    /// 音源ファイルの出力タイプ
    format: EOutputFileFormat,
    /// 音源ファイル名
    /// もし`.wav`が最後についていなければ、自動で`.wav`をファイル名につけて適用する。
    file_name: String,
    /// `true`ならファイル名の`.wav`の前にファイル出力の時間を`%Y-%m-%d_%H%m%s`形式で追加する。
    add_date_time: bool,
}

#[derive(Debug)]
pub struct OutputFileProcessData {
    common: ProcessControlItem,
    info: MetaOutputFileInfo,
}

const INPUT_IN: &'static str = "in";

impl TSystemCategory for OutputFileProcessData {
    fn get_dependent_system_categories() -> ESystemCategoryFlag {
        system_category::FILE_IO_SYSTEM
    }
}

impl TProcessItem for OutputFileProcessData {
    fn can_create_item(_setting: &ProcessItemCreateSetting) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_item(
        setting: &ProcessItemCreateSetting,
        system_setting: &InitializeSystemAccessor,
    ) -> anyhow::Result<TProcessItemPtr> {
        // これで関数実行は行うようにするけど変数は受け取らないことができる。
        let _is_ok = Self::can_create_item(&setting)?;

        if let ENode::OutputFile(v) = setting.node {
            let item = Self {
                common: ProcessControlItem::new(ProcessControlItemSetting {
                    specifier: ENodeSpecifier::OutputFile,
                    systems: &system_setting,
                }),
                info: v.clone(),
            };

            return Ok(SItemSPtr::new(item));
        };

        unreachable!("Unexpected branch");
    }
}

impl TPinCategory for OutputFileProcessData {
    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの入力側のピンの名前を返す。
    fn get_input_pin_names() -> Vec<&'static str> {
        vec![INPUT_IN]
    }

    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの出力側のピンの名前を返す。
    fn get_output_pin_names() -> Vec<&'static str> {
        vec![]
    }

    /// 関係ノードに書いているピンのカテゴリ（複数可）を返す。
    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(pin_category::BUFFER_MONO | pin_category::BUFFER_STEREO),
            _ => None,
        }
    }

    /// Inputピンのコンテナフラグ
    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(input::container_category::OUTPUT_FILE),
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

        let file_name = self.get_applied_file_name();
        let format = self.info.format.clone();
        let systems = self.common.systems.clone();

        {
            let input = self.common.get_input_internal_mut(INPUT_IN).unwrap();
            match input.output_file().unwrap() {
                EOutputFileInput::Mono(v) => {
                    process_mono(systems, format, v.sample_rate, v.buffer.clone(), file_name);
                }
                EOutputFileInput::Stereo(v) => {
                    process_stereo(systems, format, v.sample_rate, v.ch_left.clone(), v.ch_right.clone(), file_name);
                }
            };
        }

        // 状態変更。
        self.common.state = EProcessState::Finished;
    }

    fn get_applied_file_name(&self) -> String {
        // 最後の`.wav`を切り取る
        let mut file_name = match self.info.file_name.rfind(".wav") {
            None => self.info.file_name.clone(),
            Some(i) => self.info.file_name.split_at(i).0.to_string(),
        };

        // オプション
        if self.info.add_date_time {
            file_name.push_str(&Local::now().format(" %Y-%m-%d %H%M%S").to_string());
        }

        file_name.push_str(".wav");
        file_name
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
        self.common.process_input_pins_deprecated();

        match self.common.state {
            EProcessState::Stopped | EProcessState::Playing => self.update_state(input),
            _ => (),
        }
    }
}

fn process_mono(
    systems: InitializeSystemAccessor,
    format: EOutputFileFormat,
    in_sample_rate: usize,
    buffer: Vec<UniformedSample>,
    file_name: String,
) {
    let container = match format {
        EOutputFileFormat::WavLPCM16 { sample_rate } => {
            // もしsettingのsampling_rateがoutputのsampling_rateと違ったら、リサンプリングをしなきゃならない。
            let dest_sample_rate = sample_rate as f64;
            let processed_container = {
                let pitch_rate = (in_sample_rate as f64) / dest_sample_rate;
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
            .build_mono(processed_container)
            .unwrap()
        }
    };

    // 書き込み。
    systems.access_file_io_fn(move |system| {
        // 書き込み。
        let file_setting = EFileAccessSetting::Write { path: file_name };
        let file_handle = system.create_handle(file_setting);
        let mut writer = file_handle.try_write().unwrap();
        container.write(&mut writer);
    });
}

fn process_stereo(
    systems: InitializeSystemAccessor,
    format: EOutputFileFormat,
    in_sample_rate: usize,
    ch_left: Vec<UniformedSample>,
    ch_right: Vec<UniformedSample>,
    file_name: String,
) {
    let source_sample_rate = in_sample_rate as f64;

    let container = match format {
        EOutputFileFormat::WavLPCM16 { sample_rate } => {
            // もしsettingのsampling_rateがoutputのsampling_rateと違ったら、リサンプリングをしなきゃならない。
            let dest_sample_rate = sample_rate as f64;
            let pitch_rate = source_sample_rate / dest_sample_rate;

            // Left Right 全部それぞれPitchShiftする。
            let (left, right) = {
                if pitch_rate == 1.0 {
                    (ch_left, ch_right)
                } else {
                    let left = PitchShifterBuilder::default()
                        .pitch_rate(pitch_rate)
                        .window_size(128)
                        .window_function(EWindowFunction::None)
                        .build()
                        .unwrap()
                        .process_with_buffer(&PitchShifterBufferSetting { buffer: &ch_left })
                        .unwrap();
                    let right = PitchShifterBuilder::default()
                        .pitch_rate(pitch_rate)
                        .window_size(128)
                        .window_function(EWindowFunction::None)
                        .build()
                        .unwrap()
                        .process_with_buffer(&PitchShifterBufferSetting { buffer: &ch_right })
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

    systems.access_file_io_fn(move |system| {
        // 書き込み。
        let file_setting = EFileAccessSetting::Write { path: file_name };
        let file_handle = system.create_handle(file_setting);
        let mut writer = file_handle.try_write().unwrap();
        container.write(&mut writer);
    });
}

// ----------------------------------------------------------------------------
// EOutputFileInput
// ----------------------------------------------------------------------------

/// [`EProcessInputContainer::OutputFile`]の内部コンテナ
#[derive(Debug)]
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
                *self = Self::Mono(BufferMonoDynamicItem::new(0));
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
