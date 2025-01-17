use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::setting::Setting;
use crate::carg::v2::meta::system::{InitializeSystemAccessor, TSystemCategory};
use crate::carg::v2::meta::tick::TTimeTickCategory;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::node::common::{EProcessState, ProcessControlItem, ProcessControlItemSetting};
use crate::carg::v2::{
    EProcessOutput, ProcessItemCreateSetting, ProcessOutputBuffer, ProcessProcessorInput, SItemSPtr, TProcess,
    TProcessItem, TProcessItemPtr,
};
use crate::nz_define_time_tick_for;
use crate::wave::sample::UniformedSample;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MetaSeparatorInfo {}

#[derive(Debug)]
pub struct MixSeparatorProcessData {
    setting: Setting,
    common: ProcessControlItem,
    info: MetaSeparatorInfo,
}

const INPUT_IN: &'static str = "in";
const OUTPUT_OUT_1: &'static str = "out_1";
const OUTPUT_OUT_2: &'static str = "out_2";

impl TPinCategory for MixSeparatorProcessData {
    fn get_input_pin_names() -> Vec<&'static str> {
        vec![INPUT_IN]
    }

    fn get_output_pin_names() -> Vec<&'static str> {
        vec![OUTPUT_OUT_1, OUTPUT_OUT_2]
    }

    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(pin_category::BUFFER_STEREO),
            OUTPUT_OUT_1 => Some(pin_category::BUFFER_MONO),
            OUTPUT_OUT_2 => Some(pin_category::BUFFER_MONO),
            _ => None,
        }
    }

    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(input::container_category::BUFFER_STEREO_DYNAMIC),
            _ => None,
        }
    }
}

impl TProcess for MixSeparatorProcessData {
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
            EProcessState::Stopped | EProcessState::Playing => {
                self.update_state(input);

                // 自分を終わるかしないかのチェック
                if input.is_children_all_finished() {
                    self.common.state = EProcessState::Finished;
                } else {
                    self.common.state = EProcessState::Playing;
                }
            }
            _ => (),
        }
    }
}

impl TSystemCategory for MixSeparatorProcessData {}
nz_define_time_tick_for!(MixSeparatorProcessData, true, true);

impl TProcessItem for MixSeparatorProcessData {
    fn can_create_item(_setting: &ProcessItemCreateSetting) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_item(
        setting: &ProcessItemCreateSetting,
        system_setting: &InitializeSystemAccessor,
    ) -> anyhow::Result<TProcessItemPtr> {
        if let ENode::MixSeparator(v) = setting.node {
            let item = Self {
                setting: setting.setting.clone(),
                common: ProcessControlItem::new(ProcessControlItemSetting {
                    specifier: ENodeSpecifier::MixSeparator,
                    systems: &system_setting,
                }),
                info: v.clone(),
            };

            return Ok(SItemSPtr::new(item));
        };

        unreachable!("Unexpected node type");
    }
}

impl MixSeparatorProcessData {
    fn update_state(&mut self, in_input: &ProcessProcessorInput) {
        let (out_1, out_2, sample_rate) = {
            let mut item = self.common.get_input_internal_mut(INPUT_IN).unwrap();
            let item = item.buffer_stereo_dynamic_mut().unwrap();

            // 毎フレーム次のノードにつぎ込む。
            // もし尺が一致していなくて、誤差があるなら長い分に合わせる。
            if item.ch_left.is_empty() && item.ch_right.is_empty() {
                return;
            }

            // Get absolute offset of two ch_left and ch_right.
            let offset = ((item.ch_left.len() as isize) - (item.ch_right.len() as isize)).abs() as usize;

            // 足りないところにオフセット分を足す。
            let ideal_size = item.ch_left.len().min(item.ch_right.len()) + offset;
            let mut out_1 = item.ch_left.drain(..).collect_vec();
            out_1.resize(ideal_size, UniformedSample::MIN);
            let mut out_2 = item.ch_right.drain(..).collect_vec();
            out_2.resize(ideal_size, UniformedSample::MIN);

            (out_1, out_2, item.sample_rate)
        };

        self.common
            .insert_to_output_pin(
                OUTPUT_OUT_1,
                EProcessOutput::BufferMono(ProcessOutputBuffer::new(out_1, sample_rate)),
            )
            .unwrap();
        self.common
            .insert_to_output_pin(
                OUTPUT_OUT_2,
                EProcessOutput::BufferMono(ProcessOutputBuffer::new(out_2, sample_rate)),
            )
            .unwrap();
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
