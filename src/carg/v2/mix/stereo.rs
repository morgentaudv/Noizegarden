use itertools::Itertools;
use serde::{Deserialize, Serialize};
use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::{EProcessOutput, ProcessControlItem, ProcessItemCreateSetting, ProcessOutputBufferStereo, ProcessProcessorInput, SItemSPtr, Setting, TProcess, TProcessItem, TProcessItemPtr};
use crate::carg::v2::meta::sample_timer::SampleTimer;
use crate::carg::v2::meta::system::{InitializeSystemAccessor, TSystemCategory};
use crate::carg::v2::node::common::{EProcessState, ProcessControlItemSetting};
use crate::math::float::EFloatCommonPin;
use crate::wave::sample::UniformedSample;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaStereoInfo {
    pub gain_0: EFloatCommonPin,
    pub gain_1: EFloatCommonPin,
}

/// モノラルをステレオに変換する
#[derive(Debug)]
pub struct MixStereoProcessData {
    setting: Setting,
    common: ProcessControlItem,
    info: MetaStereoInfo,
    timer: SampleTimer,
}

const INPUT_IN_1: &'static str = "in_1";
const INPUT_IN_2: &'static str = "in_2";
const OUTPUT_OUT: &'static str = "out";

impl TPinCategory for MixStereoProcessData {
    fn get_input_pin_names() -> Vec<&'static str> {
        vec![INPUT_IN_1, INPUT_IN_2]
    }

    fn get_output_pin_names() -> Vec<&'static str> {
        vec![OUTPUT_OUT]
    }

    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            INPUT_IN_1 => Some(pin_category::BUFFER_MONO),
            INPUT_IN_2 => Some(pin_category::BUFFER_MONO),
            OUTPUT_OUT => Some(pin_category::BUFFER_STEREO),
            _ => None,
        }
    }

    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            INPUT_IN_1 => Some(input::container_category::BUFFER_MONO_DYNAMIC),
            INPUT_IN_2 => Some(input::container_category::BUFFER_MONO_DYNAMIC),
            _ => None,
        }
    }
}

impl TProcessItem for MixStereoProcessData {
    fn can_create_item(_setting: &ProcessItemCreateSetting) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_item(setting: &ProcessItemCreateSetting, system_setting: &InitializeSystemAccessor) -> anyhow::Result<TProcessItemPtr> {
        match setting.node {
            ENode::MixStereo(v) => {
                let item = Self {
                    setting: setting.setting.clone(),
                    common: ProcessControlItem::new(ProcessControlItemSetting {
                        specifier: ENodeSpecifier::MixStereo,
                        systems: &system_setting,
                    }),
                    info: v.clone(),
                    timer: SampleTimer::new(0.0),
                };

                Ok(SItemSPtr::new(item))
            }
            _ => unreachable!("Unexpected node type"),
        }
    }
}

impl MixStereoProcessData {
    fn update_state(&mut self, input: &ProcessProcessorInput) {
        // まずRealtimeだけで。
        // @todo OFFLINE用はバッチにしたい。

        // Inputがあるかを確認する。なければ無視。
        let sample_rate_1 = {
            let input_internal = self.common.get_input_internal(INPUT_IN_1).unwrap();
            let input = input_internal.buffer_mono_dynamic().unwrap();
            // もしインプットがきてなくて、Fsがセットされたなきゃなんもしない。
            if input.sample_rate == 0 {
                return;
            }

            input.sample_rate
        };
        let sample_rate_2 = {
            let input_internal = self.common.get_input_internal(INPUT_IN_2).unwrap();
            let input = input_internal.buffer_mono_dynamic().unwrap();
            // もしインプットがきてなくて、Fsがセットされたなきゃなんもしない。
            if input.sample_rate == 0 {
                return;
            }

            input.sample_rate
        };
        debug_assert_eq!(sample_rate_1, sample_rate_2);

        let time_result = self.timer.process_time(input.common.frame_time, sample_rate_1);
        if time_result.required_sample_count <= 0 {
            return;
        }

        // タイマーがまだ動作前なら何もしない。
        let old_internal_time = time_result.old_time;
        if self.timer.internal_time() <= 0.0 {
            // ゼロ入りのバッファだけを作る。
            let buffer = vec![UniformedSample::MIN; time_result.required_sample_count];
            self.common
                .insert_to_output_pin(
                    OUTPUT_OUT,
                    EProcessOutput::BufferStereo(ProcessOutputBufferStereo{
                        ch_left: buffer.clone(),
                        ch_right: buffer,
                        sample_rate: sample_rate_1,
                    }),
                )
                .unwrap();

            self.common.state = EProcessState::Playing;
            return;
        }

        // sample_countsからバッファ分をとる。
        // もしたりなきゃ作って返す。
        let pre_blank_counts = if old_internal_time < 0.0 {
            ((old_internal_time * -1.0) * (sample_rate_1 as f64)).floor() as usize
        } else {
            0
        };
        debug_assert!(time_result.required_sample_count >= pre_blank_counts);

        // 処理したものを渡す。
        let result_1 = self.drain_buffer(input, time_result.required_sample_count, pre_blank_counts, INPUT_IN_1);
        let result_2 = self.drain_buffer(input, time_result.required_sample_count, pre_blank_counts, INPUT_IN_2);

        // outputのどこかに保持する。
        self.common
            .insert_to_output_pin(
                OUTPUT_OUT,
                EProcessOutput::BufferStereo(ProcessOutputBufferStereo{
                    ch_left: result_1.buffer,
                    ch_right: result_2.buffer,
                    sample_rate: sample_rate_1,
                }),
            )
            .unwrap();

        if result_1.is_finished && result_2.is_finished && input.is_children_all_finished() {
            self.common.state = EProcessState::Finished;
            return;
        } else {
            self.common.state = EProcessState::Playing;
            return;
        }
    }

    fn drain_buffer(
        &mut self,
        in_input: &ProcessProcessorInput,
        sample_counts: usize,
        pre_blank_counts: usize,
        pin: &str
    ) -> DrainBufferResult {
        debug_assert!(sample_counts >= pre_blank_counts);
        let mut input_internal = self.common.get_input_internal_mut(pin).unwrap();
        let input = input_internal.buffer_mono_dynamic_mut().unwrap();

        // `pre_blank_counts`が0より大きければバッファを作る。
        let mut buffer = vec![];
        if pre_blank_counts > 0 {
            buffer.resize(pre_blank_counts, UniformedSample::MIN);
        }
        let sample_counts = sample_counts - pre_blank_counts;

        // バッファ0補充分岐
        let now_buffer_len = input.buffer.len();
        let is_buffer_enough = now_buffer_len >= sample_counts;
        if !is_buffer_enough {
            buffer.append(&mut input.buffer.drain(..).collect_vec());
            buffer.resize(sample_counts, UniformedSample::MIN);
        }
        else {
            buffer.append(&mut input.buffer.drain(..sample_counts).collect_vec());
        }

        DrainBufferResult {
            buffer,
            is_finished: !is_buffer_enough && in_input.is_children_all_finished(),
        }
    }
}

#[derive(Default)]
struct DrainBufferResult {
    buffer: Vec<UniformedSample>,
    is_finished: bool,
}

impl TSystemCategory for MixStereoProcessData {}


impl TProcess for MixStereoProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn can_process(&self) -> bool {
        true
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
