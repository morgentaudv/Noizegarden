use serde::{Deserialize, Serialize};
use crate::carg::v2::{EProcessState, ProcessControlItem, ProcessItemCreateSetting, ProcessProcessorInput, TProcess, TProcessItem, TProcessItemPtr};
use crate::carg::v2::meta::{input, pin_category, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::system::{system_category, ESystemCategoryFlag, TSystemCategory};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaOutputDeviceInfo {

}

#[derive(Debug)]
pub struct OutputDeviceProcessData {
    common: ProcessControlItem,
}

const INPUT_IN: &'static str = "in";

impl TPinCategory for OutputDeviceProcessData {
    fn get_input_pin_names() -> Vec<&'static str> {
        vec![INPUT_IN]
    }

    fn get_output_pin_names() -> Vec<&'static str> {
        vec![]
    }

    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(pin_category::BUFFER_MONO),
            _ => None,
        }
    }

    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(input::container_category::OUTPUT_DEVICE),
            _ => None,
        }
    }
}

impl TSystemCategory for OutputDeviceProcessData {
    fn get_dependent_system_categories() -> ESystemCategoryFlag {
        system_category::AUDIO_DEVICE
    }
}

impl TProcess for OutputDeviceProcessData {
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
        todo!()
    }
}

impl TProcessItem for OutputDeviceProcessData {
    fn can_create_item(_setting: &ProcessItemCreateSetting) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_item(setting: &ProcessItemCreateSetting) -> anyhow::Result<TProcessItemPtr> {
        todo!()
    }
}


// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
