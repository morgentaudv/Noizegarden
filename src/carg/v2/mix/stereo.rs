use itertools::Itertools;
use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::{EProcessOutput, EProcessState, EmitterRange, ProcessControlItem, ProcessOutputBuffer, ProcessOutputBufferStereo, ProcessProcessorInput, SItemSPtr, Setting, TProcess, TProcessItemPtr};
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::output::EProcessOutputContainer;

/// モノラルをステレオに変換する
#[derive(Debug)]
pub struct MixStereoProcessData {
    setting: Setting,
    common: ProcessControlItem,
    gain_0: f64,
    gain_1: f64,
}

impl TPinCategory for MixStereoProcessData {
    fn get_input_pin_names() -> Vec<&'static str> {
        vec!["in_1", "in_2"]
    }

    fn get_output_pin_names() -> Vec<&'static str> {
        vec!["out"]
    }

    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            "in_1" | "in_2" => Some(pin_category::BUFFER_MONO),
            "out" => Some(pin_category::BUFFER_STEREO),
            _ => None,
        }
    }

    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            "in_1" | "in_2" => Some(input::container_category::BUFFER_MONO_PHANTOM),
            _ => None,
        }
    }
}

impl MixStereoProcessData {
    pub fn create_from(node: &ENode, setting: &Setting) -> TProcessItemPtr {
        match node {
            ENode::MixStereo{ gain_0, gain_1 } => {
                let item = Self {
                    setting: setting.clone(),
                    common: ProcessControlItem::new(ENodeSpecifier::MixStereo),
                    gain_0: 0.707,
                    gain_1: 0.707,
                };
                SItemSPtr::new(item)
            }
            _ => unreachable!("Unexpected node type"),
        }
    }

    fn update_state(&mut self, in_input: &ProcessProcessorInput) {
        let left_buffer = {
            let left_output_pin = self
                .common
                .get_input_pin("in_1")
                .unwrap()
                .upgrade()
                .unwrap()
                .borrow()
                .linked_pins
                .first()
                .unwrap()
                .upgrade()
                .unwrap();
            let borrowed = left_output_pin.borrow();
            match &borrowed.output {
                EProcessOutputContainer::BufferMono(v) => v.buffer.clone(),
                _ => unreachable!("Unexpected branch"),
            }
        };

        let right_buffer = {
            let left_output_pin = self
                .common
                .get_input_pin("in_2")
                .unwrap()
                .upgrade()
                .unwrap()
                .borrow()
                .linked_pins
                .first()
                .unwrap()
                .upgrade()
                .unwrap();
            let borrowed = left_output_pin.borrow();
            match &borrowed.output {
                EProcessOutputContainer::BufferMono(v) => v.buffer.clone(),
                _ => unreachable!("Unexpected branch"),
            }
        };

        // outputのどこかに保持する。
        self.common
            .insert_to_output_pin(
                "out",
                EProcessOutput::BufferStereo(ProcessOutputBufferStereo{
                    ch_left: left_buffer,
                    ch_right: right_buffer,
                    setting: self.setting.clone(),
                    range: EmitterRange{ start: 0.0, length: 0.0 },
                }),
            )
            .unwrap();

        if in_input.is_children_all_finished() {
            self.common.state = EProcessState::Finished;
            return;
        } else {
            self.common.state = EProcessState::Playing;
            return;
        }
    }
}

impl TProcess for MixStereoProcessData {
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
