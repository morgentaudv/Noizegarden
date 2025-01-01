use serde::{Deserialize, Serialize};
use crate::carg::v2::{ProcessControlItem, ProcessItemCreateSetting, ProcessProcessorInput, SItemSPtr, TProcess, TProcessItem, TProcessItemPtr};
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::setting::Setting;
use crate::carg::v2::meta::system::{InitializeSystemAccessor, TSystemCategory};
use crate::carg::v2::meta::tick::TTimeTickCategory;
use crate::carg::v2::node::common::{EProcessState, ProcessControlItemSetting};
use crate::nz_define_time_tick_for;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaIRConvInfo {

}

#[derive(Debug)]
pub struct IRConvolutionProcessData {
    setting: Setting,
    common: ProcessControlItem,
    info: MetaIRConvInfo,
}

const INPUT_SOURCE: &'static str = "in_source";
const INPUT_IR: &'static str = "in_ir";
const OUTPUT_OUT: &'static str = "out";

impl TPinCategory for IRConvolutionProcessData {
    fn get_input_pin_names() -> Vec<&'static str> {
        vec![INPUT_SOURCE, INPUT_IR]
    }

    fn get_output_pin_names() -> Vec<&'static str> {
        vec![OUTPUT_OUT]
    }

    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            INPUT_SOURCE => Some(pin_category::BUFFER_MONO),
            INPUT_IR => Some(pin_category::BUFFER_MONO),
            OUTPUT_OUT => Some(pin_category::BUFFER_MONO),
            _ => None,
        }
    }

    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            INPUT_SOURCE => Some(input::container_category::BUFFER_MONO_DYNAMIC),
            INPUT_IR => Some(input::container_category::BUFFER_MONO_PHANTOM),
            _ => None,
        }
    }
}

impl TSystemCategory for IRConvolutionProcessData {}

nz_define_time_tick_for!(IRConvolutionProcessData, true, true);

impl TProcess for IRConvolutionProcessData {
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

impl TProcessItem for IRConvolutionProcessData {
    fn can_create_item(_setting: &ProcessItemCreateSetting) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_item(setting: &ProcessItemCreateSetting, system_setting: &InitializeSystemAccessor) -> anyhow::Result<TProcessItemPtr> {
        if let ENode::FilterIRConvolution(v) = setting.node {
            let item = Self {
                setting: setting.setting.clone(),
                common: ProcessControlItem::new(ProcessControlItemSetting {
                    specifier: ENodeSpecifier::FilterIRConvolution,
                    systems: &system_setting,
                }),
                info: v.clone(),
            };

            return Ok(SItemSPtr::new(item));
        }

        unreachable!("Unexpected branch");
    }
}

impl IRConvolutionProcessData {
    fn update_state(&self, _in_input: &ProcessProcessorInput) {
        todo!()
    }
}


// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
