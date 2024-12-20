use serde::{Deserialize, Serialize};
use crate::carg::v2::meta::setting::Setting;
use crate::carg::v2::meta::{input, pin_category, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::system::TSystemCategory;
use crate::carg::v2::node::common::ProcessControlItem;
use crate::carg::v2::meta::tick::TTimeTickCategory;
use crate::carg::v2::{ProcessItemCreateSetting, ProcessItemCreateSettingSystem, ProcessProcessorInput, TProcess, TProcessItem, TProcessItemPtr};
use crate::nz_define_time_tick_for;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaResampleInfo {
    /// サンプルレートに変換
    pub to_sample_rate: usize,
    ///
    pub high_quality: bool,
}

#[derive(Debug)]
pub struct ResampleProcessData {
    setting: Setting,
    common: ProcessControlItem,
    info: MetaResampleInfo,
}

const INPUT_IN: &'static str = "in";
const OUTPUT_OUT: &'static str = "out";

impl TPinCategory for ResampleProcessData {
    fn get_input_pin_names() -> Vec<&'static str> {
        vec![INPUT_IN]
    }

    fn get_output_pin_names() -> Vec<&'static str> {
        vec![OUTPUT_OUT]
    }

    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(pin_category::BUFFER_MONO),
            OUTPUT_OUT => Some(pin_category::BUFFER_MONO),
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

impl TSystemCategory for ResampleProcessData {}
nz_define_time_tick_for!(ResampleProcessData, true, true);

impl TProcess for ResampleProcessData {
    fn is_finished(&self) -> bool {
        todo!()
    }

    fn can_process(&self) -> bool {
        todo!()
    }

    fn get_common_ref(&self) -> &ProcessControlItem {
        todo!()
    }

    fn get_common_mut(&mut self) -> &mut ProcessControlItem {
        todo!()
    }

    fn try_process(&mut self, input: &ProcessProcessorInput) {
        todo!()
    }
}

impl TProcessItem for ResampleProcessData {
    fn can_create_item(_setting: &ProcessItemCreateSetting) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_item(setting: &ProcessItemCreateSetting, system_setting: &ProcessItemCreateSettingSystem) -> anyhow::Result<TProcessItemPtr> {
        todo!()
    }
}

impl ResampleProcessData {

}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
