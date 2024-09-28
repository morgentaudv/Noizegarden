use itertools::Itertools;
use crate::carg::v2::meta::input::{EInputContainerCategoryFlag, EProcessInputContainer};
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::{EProcessOutput, EProcessState, ProcessControlItem, ProcessOutputFrequency, ProcessOutputText, ProcessProcessorInput, SItemSPtr, Setting, TProcess, TProcessItemPtr};
use crate::math::window::EWindowFunction;
use crate::wave::analyze::analyzer::{FrequencyAnalyzerV2, WaveContainerSetting};
use crate::wave::analyze::method::EAnalyzeMethod;
use crate::wave::sample::UniformedSample;

#[derive(Debug)]
pub struct AnalyzerFFTProcessData {
    common: ProcessControlItem,
    level: usize,
    window_function: EWindowFunction,
    /// 半分ずつ重ねるか
    overlap: bool,
}

const INPUT_IN: &'static str = "in";
const OUTPUT_INFO: &'static str = "out_info";
const OUTPUT_FREQ: &'static str = "out_freq";

impl TPinCategory for AnalyzerFFTProcessData {
    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの入力側のピンの名前を返す。
    fn get_input_pin_names() -> Vec<&'static str> {
        vec![INPUT_IN]
    }

    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの出力側のピンの名前を返す。
    fn get_output_pin_names() -> Vec<&'static str> {
        vec![OUTPUT_INFO, OUTPUT_FREQ]
    }

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

impl AnalyzerFFTProcessData {
    pub fn create_from(node: &ENode, _setting: &Setting) -> TProcessItemPtr {
        match node {
            ENode::AnalyzerFFT { level, window_function, overlap } => {
                let item= Self {
                    common: ProcessControlItem::new(ENodeSpecifier::AnalyzerFFT),
                    level: *level,
                    window_function: *window_function,
                    overlap: *overlap,
                };
                SItemSPtr::new(item)
            }
            _ => unreachable!("Unexpected branch."),
        }
    }

    fn drain_buffer(&mut self, in_input: &ProcessProcessorInput) -> (Vec<UniformedSample>, u64) {
        // チェックしてself.levelよりバッファが多くないと処理しない。
        let mut now_buffer_len = 0usize;
        let is_buffer_enough = match &*self.common.get_input_internal(INPUT_IN).unwrap() {
            EProcessInputContainer::BufferMonoDynamic(v) => {
                now_buffer_len = v.buffer.len();
                now_buffer_len >= self.level
            }
            _ => false,
        };

        let mut item = self.common.get_input_internal_mut(INPUT_IN).unwrap();
        let v = &mut item.buffer_mono_dynamic_mut().unwrap();
        let sample_rate = v.setting.as_ref().unwrap().sample_rate;

        // バッファ0補充分岐
        if !is_buffer_enough && in_input.is_children_all_finished() {
            return if self.overlap {
                let ideal_drain_samples = self.level >> 1;
                let mut buffer = vec![];

                buffer.append(&mut v.buffer.drain(..ideal_drain_samples.min(now_buffer_len)).collect());
                let remained_samples = buffer.len();
                buffer.append(
                    &mut v
                        .buffer
                        .iter()
                        .take(remained_samples.min(ideal_drain_samples))
                        .copied()
                        .collect_vec(),
                );
                buffer.resize(self.level, UniformedSample::MIN);

                (buffer, sample_rate)
            } else {
                let mut buffer = v.buffer.drain(..now_buffer_len).collect_vec();
                buffer.resize(self.level, UniformedSample::MIN);
                (buffer, sample_rate)
            };
        }

        // 通常分岐
        if self.overlap {
            // 全体のdrainが使えない。前半分はdrainできるけど、残り半分はコピーする必要がある。
            let drain_samples = self.level >> 1;
            let mut buffer = v.buffer.drain(..drain_samples).collect_vec();

            // そして残りの半分をコピーしてbufferに追加する。
            buffer.append(&mut v.buffer.iter().take(drain_samples).copied().collect_vec());
            (buffer, sample_rate)
        } else {
            let buffer = v.buffer.drain(..self.level).collect_vec();
            (buffer, sample_rate)
        }
    }

    fn update_state(&mut self, in_input: &ProcessProcessorInput) {
        // チェックしてself.levelよりバッファが多くないと処理しない。
        let is_buffer_enough = match &*self.common.get_input_internal(INPUT_IN).unwrap() {
            EProcessInputContainer::BufferMonoDynamic(v) => v.buffer.len() >= self.level,
            _ => false,
        };
        if !is_buffer_enough && !in_input.is_children_all_finished() {
            return;
        }

        // もし前ノードからの更新はこれ以上ないはずなのに、バッファがたりなきゃ、
        // 0値の余裕分を用意する。
        let (buffer, sample_rate) = self.drain_buffer(in_input);

        // このノードでは最初からADを行う。
        // もし尺が足りなければ、そのまま終わる。
        // inputのSettingのsample_rateから各バッファのサンプルの発生時間を計算する。
        let samples_count = self.level;
        let frequencies = {
            let analyzer = FrequencyAnalyzerV2 {
                analyze_method: EAnalyzeMethod::FFT,
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
                        overlap: false,
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

impl TProcess for AnalyzerFFTProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn can_process(&self) -> bool { true }

    fn get_common_ref(&self) -> &ProcessControlItem {
        &self.common
    }

    fn get_common_mut(&mut self) -> &mut ProcessControlItem {
        &mut self.common
    }

    fn try_process(&mut self, input: &ProcessProcessorInput) {
        let has_buffer = match &*self.common.get_input_internal(INPUT_IN).unwrap() {
            EProcessInputContainer::BufferMonoDynamic(v) => v.buffer.len() >= self.level,
            _ => false,
        };
        if !(self.common.is_all_input_pins_update_notified() | input.is_children_all_finished() | has_buffer) {
            return;
        }

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
