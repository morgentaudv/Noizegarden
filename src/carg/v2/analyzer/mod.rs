use super::{
    ENode, EProcessOutput, EProcessState, ProcessControlItem, ProcessOutputText, ProcessProcessorInput, SItemSPtr,
    Setting, TProcess, TProcessItemPtr,
};
use crate::carg::v2::meta::input::EProcessInputContainer;
use crate::carg::v2::meta::ENodeSpecifier;
use crate::wave::analyze::{
    analyzer::{FrequencyAnalyzerV2, WaveContainerSetting},
    method::EAnalyzeMethod,
    window::EWindowFunction,
};
use itertools::Itertools;

#[derive(Debug)]
pub struct AnalyzerDFSProcessData {
    common: ProcessControlItem,
    level: usize,
}

impl AnalyzerDFSProcessData {
    pub fn create_from(node: &ENode, setting: &Setting) -> TProcessItemPtr {
        match node {
            ENode::AnalyzerDFT { level } => {
                let item = Self::new(*level);
                SItemSPtr::new(item)
            }
            _ => unreachable!("Unexpected branch."),
        }
    }

    fn new(level: usize) -> Self {
        Self {
            common: ProcessControlItem::new(ENodeSpecifier::AnalyzerDFT),
            level,
        }
    }

    fn update_state(&mut self, in_input: &ProcessProcessorInput) {
        // チェックしてself.levelよりバッファが多くないと処理しない。
        let can_process = match &*self.common.get_input_internal("in").unwrap() {
            EProcessInputContainer::WaveBuffersDynamic(v) => v.buffer.len() >= self.level,
            _ => false,
        };
        if !can_process {
            return;
        }

        let (buffer, sample_rate) = match &mut *self.common.get_input_internal_mut("in").unwrap() {
            EProcessInputContainer::WaveBuffersDynamic(v) => {
                let buffer = v.buffer.drain(..self.level).collect_vec();
                (buffer, v.setting.as_ref().unwrap().sample_rate)
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
                window_function: EWindowFunction::None,
            };

            let setting = WaveContainerSetting {
                container: &buffer,
                start_sample_index: 0,
                samples_count,
            };
            analyzer.analyze_container(&setting).unwrap()
        };

        // out_info関連出力処理
        if self.common.is_output_pin_connected("out_info") {
            let mut log = "".to_owned();
            for frequency in frequencies {
                if frequency.amplitude < 5.0 {
                    continue;
                }

                log += &format!("(Freq: {:.0}, Amp: {:.2}) ", frequency.frequency, frequency.amplitude);
            }

            self.common
                .insert_to_output_pin("out_info", EProcessOutput::Text(ProcessOutputText { text: log }))
                .unwrap();
        }

        // out_freq関連出力処理
        if self.common.is_output_pin_connected("out_freq") {

        }

        // 状態変更。
        if in_input.is_children_all_finished() {
            self.common.state = EProcessState::Finished;
            return;
        } else {
            self.common.state = EProcessState::Playing;
            return;
        }
    }
}

impl TProcess for AnalyzerDFSProcessData {
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

    fn try_process(&mut self, input: &super::ProcessProcessorInput) {
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
