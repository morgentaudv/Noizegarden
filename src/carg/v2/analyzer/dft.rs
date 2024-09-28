use crate::carg::v2::meta::input::{EInputContainerCategoryFlag, EProcessInputContainer};
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::wave::analyze::{
    analyzer::{FrequencyAnalyzerV2, WaveContainerSetting},
    method::EAnalyzeMethod,
};
use itertools::Itertools;
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::{EProcessOutput, EProcessState, ProcessControlItem, ProcessOutputFrequency, ProcessOutputText, ProcessProcessorInput, SItemSPtr, Setting, TProcess, TProcessItemPtr};
use crate::math::window::EWindowFunction;

#[derive(Debug)]
pub struct AnalyzerDFTProcessData {
    common: ProcessControlItem,
    level: usize,
    window_function: EWindowFunction,
    /// 半分ずつ重ねるか
    overlap: bool,
}

const INPUT_IN: &'static str = "in";
const OUTPUT_INFO: &'static str = "out_info";
const OUTPUT_FREQ: &'static str = "out_freq";

impl TPinCategory for AnalyzerDFTProcessData {
    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの入力側のピンの名前を返す。
    fn get_input_pin_names() -> Vec<&'static str> { vec![INPUT_IN] }

    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの出力側のピンの名前を返す。
    fn get_output_pin_names() -> Vec<&'static str> { vec![OUTPUT_INFO, OUTPUT_FREQ] }

    /// 関係ノードに書いているピンのカテゴリ（複数可）を返す。
    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(pin_category::BUFFER_MONO),
            OUTPUT_INFO => Some(pin_category::TEXT),
            OUTPUT_FREQ => Some(pin_category::FREQUENCY),
            _ => None,
        }
    }

    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(input::container_category::BUFFER_MONO_DYNAMIC),
            _ => None,
        }
    }
}

impl AnalyzerDFTProcessData {
    pub fn create_from(node: &ENode, _setting: &Setting) -> TProcessItemPtr {
        match node {
            ENode::AnalyzerDFT { level, window_function, overlap } => {
                let item = Self {
                    common: ProcessControlItem::new(ENodeSpecifier::AnalyzerDFT),
                    level: *level,
                    window_function: *window_function,
                    overlap: *overlap,
                };
                SItemSPtr::new(item)
            }
            _ => unreachable!("Unexpected branch."),
        }
    }

    fn update_state(&mut self, in_input: &ProcessProcessorInput) {
        // チェックしてself.levelよりバッファが多くないと処理しない。
        let can_process = match &*self.common.get_input_internal(INPUT_IN).unwrap() {
            EProcessInputContainer::BufferMonoDynamic(v) => v.buffer.len() >= self.level,
            _ => false,
        };
        if !can_process {
            return;
        }

        let (buffer, sample_rate) = match &mut *self.common.get_input_internal_mut(INPUT_IN).unwrap() {
            EProcessInputContainer::BufferMonoDynamic(v) => {
                if self.overlap {
                    // 全体のdrainが使えない。
                    // 前半分はdrainできるけど、残り半分はコピーする必要がある。
                    let drain_samples = self.level >> 1;
                    let mut buffer = v.buffer.drain(..drain_samples).collect_vec();

                    // そして残りの半分をコピーしてbufferに追加する。
                    buffer.append(&mut v.buffer.iter().take(drain_samples).copied().collect_vec());
                    (buffer, v.setting.as_ref().unwrap().sample_rate)
                }
                else {
                    let buffer = v.buffer.drain(..self.level).collect_vec();
                    (buffer, v.setting.as_ref().unwrap().sample_rate)
                }
            }
            _ => unreachable!("Unexpected input."),
        };

        // このノードでは最初からADを行う。
        // もし尺が足りなければ、そのまま終わる。
        // inputのSettingのsample_rateから各バッファのサンプルの発生時間を計算する。
        let samples_count = self.level;
        let frequencies = {
            let analyzer = FrequencyAnalyzerV2 {
                analyze_method: EAnalyzeMethod::DFT,
                frequency_start: 0.0,
                frequency_width: sample_rate as f64,
                frequency_bin_count: self.level as u32,
                window_function: self.window_function,
            };

            let setting = WaveContainerSetting {
                container: &buffer,
                start_sample_index: 0,
                samples_count,
            };
            analyzer.analyze_container(&setting).unwrap()
        };

        // out_info関連出力処理
        if self.common.is_output_pin_connected(OUTPUT_INFO) {
            let mut log = "".to_owned();
            for frequency in &frequencies {
                if frequency.amplitude < 5.0 {
                    continue;
                }

                log += &format!("(Freq: {:.0}, Amp: {:.2}) ", frequency.frequency, frequency.amplitude);
            }

            self.common
                .insert_to_output_pin(OUTPUT_INFO, EProcessOutput::Text(ProcessOutputText { text: log }))
                .unwrap();
        }

        // out_freq関連出力処理
        if self.common.is_output_pin_connected(OUTPUT_FREQ) {
            let analyzed_sample_len = self.level;

            self.common
                .insert_to_output_pin(
                    OUTPUT_FREQ,
                    EProcessOutput::Frequency(ProcessOutputFrequency {
                        frequencies,
                        analyzed_sample_len,
                        overlap: self.overlap,
                    }),
                )
                .unwrap();
        }

        // 状態変更。
        if in_input.is_children_all_finished() {
            // 24-09-28 overlapしているときには前のEmitterの処理が終わっても
            // こっちじゃまだ処理分があるので、バッファが残っている限り終わらせない。
            let can_more_process = match &*self.common.get_input_internal(INPUT_IN).unwrap() {
                EProcessInputContainer::BufferMonoDynamic(v) => v.buffer.len() >= self.level,
                _ => false,
            };
            if !can_more_process {
                self.common.state = EProcessState::Finished;
            }
            else {
                self.common.state = EProcessState::Playing;
            }

            return;
        } else {
            self.common.state = EProcessState::Playing;
            return;
        }
    }
}

impl TProcess for AnalyzerDFTProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn can_process(&self) -> bool {
        let has_buffer = match &*self.common.get_input_internal(INPUT_IN).unwrap() {
            EProcessInputContainer::BufferMonoDynamic(v) => v.buffer.len() >= self.level,
            _ => false,
        };

        self.common.is_all_input_pins_update_notified() | has_buffer
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
