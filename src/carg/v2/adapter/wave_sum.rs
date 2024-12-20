use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::output::EProcessOutputContainer;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::{
    EProcessOutput, ProcessControlItem, ProcessOutputBuffer, ProcessProcessorInput,
    SItemSPtr, Setting, TProcess, TProcessItemPtr,
};
use crate::wave::sample::UniformedSample;
use itertools::Itertools;
use crate::carg::v2::meta::system::TSystemCategory;
use crate::carg::v2::meta::tick::TTimeTickCategory;
use crate::carg::v2::node::common::EProcessState;

/// ユニット単位でADEnvelopeを生成するための時間に影響しないエミッタ。
#[derive(Debug)]
pub struct AdapterWaveSumProcessData {
    setting: Setting,
    common: ProcessControlItem,
}

impl TPinCategory for AdapterWaveSumProcessData {
    fn get_input_pin_names() -> Vec<&'static str> {
        vec!["in_1", "in_2", "in_3", "in_4", "in_5", "in_6", "in_7", "in_8", "in_9", "in_10"]
    }

    fn get_output_pin_names() -> Vec<&'static str> {
        vec!["out"]
    }

    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            "in_1" | "in_2" | "in_3" | "in_4" | "in_5" | "in_6" | "in_7" | "in_8" | "in_9" | "in_10" => {
                Some(pin_category::BUFFER_MONO)
            }
            "out" => Some(pin_category::BUFFER_MONO),
            _ => None,
        }
    }

    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            "in_1" | "in_2" | "in_3" | "in_4" | "in_5" | "in_6" | "in_7" | "in_8" | "in_9" | "in_10" => {
                Some(input::container_category::BUFFER_MONO_PHANTOM)
            }
            _ => None,
        }
    }
}

impl AdapterWaveSumProcessData {
    pub fn create_from(_node: &ENode, setting: &Setting) -> TProcessItemPtr {
        let item = Self {
            common: ProcessControlItem::new(ENodeSpecifier::AdapterWaveSum),
            setting: setting.clone(),
        };
        SItemSPtr::new(item)
    }

    fn update_state(&mut self, in_input: &ProcessProcessorInput) {
        let output_pins = Self::get_input_pin_names()
            .into_iter()
            .filter_map(|v| self.common.get_input_pin(v).unwrap().upgrade())
            .filter(|v| v.borrow().linked_pins.is_empty() == false)
            .map(|v| v.borrow().linked_pins.first().unwrap().upgrade().unwrap())
            .collect_vec();
        if output_pins.is_empty() {
            return;
        }

        let inputs = {
            let mut inputs = vec![];
            for output_pin in &output_pins {
                let borrowed = output_pin.borrow();
                let input = match &borrowed.output {
                    EProcessOutputContainer::BufferMono(v) => v.clone(),
                    _ => unreachable!("Unexpected branch"),
                };
                inputs.push(input);
            }
            inputs
        };
        debug_assert_eq!(inputs.iter().map(|v| v.sample_rate).all_equal(), true);

        let sample_rate = inputs.first().unwrap().sample_rate;
        let recep = (inputs.len() as f64).recip();
        let mut buffer = vec![];
        buffer.resize(inputs.first().unwrap().buffer.len(), UniformedSample::MIN);
        for input in inputs {
            for (i, v) in input.buffer.iter().enumerate() {
                buffer[i] += *v;
            }
        }

        for sample in &mut buffer {
            *sample = recep * *sample;
        }

        // outputのどこかに保持する。
        self.common
            .insert_to_output_pin(
                "out",
                EProcessOutput::BufferMono(ProcessOutputBuffer::new(buffer, sample_rate)),
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

impl TSystemCategory for AdapterWaveSumProcessData {}

impl TTimeTickCategory for AdapterWaveSumProcessData {
    fn can_support_offline() -> bool {
        true
    }

    fn can_support_realtime() -> bool {
        true
    }
}

impl TProcess for AdapterWaveSumProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn can_process(&self) -> bool {
        self.common.is_all_input_pins_update_notified()
    }

    /// 共用アイテムの参照を返す。
    fn get_common_ref(&self) -> &ProcessControlItem {
        &self.common
    }

    /// 共用アイテムの可変参照を返す。
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
