use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::system::{InitializeSystemAccessor, TSystemCategory};
use crate::carg::v2::meta::tick::TTimeTickCategory;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::node::common::ProcessControlItemSetting;
use crate::carg::v2::{
    ProcessControlItem, ProcessItemCreateSetting, ProcessProcessorInput, SItemSPtr, TProcess, TProcessItem,
    TProcessItemPtr,
};

/// ダミーノード
#[derive(Debug)]
pub struct DummyProcessData {
    common: ProcessControlItem,
}

impl TPinCategory for DummyProcessData {
    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの入力側のピンの名前を返す。
    fn get_input_pin_names() -> Vec<&'static str> {
        vec!["in"]
    }

    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの出力側のピンの名前を返す。
    fn get_output_pin_names() -> Vec<&'static str> {
        vec![]
    }

    /// 関係ノードに書いているピンのカテゴリ（複数可）を返す。
    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            "in" => Some(pin_category::DUMMY),
            _ => None,
        }
    }

    /// Inputピンのコンテナフラグ
    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            "in" => Some(input::container_category::DUMMY),
            _ => None,
        }
    }
}

impl TProcessItem for DummyProcessData {
    fn can_create_item(_setting: &ProcessItemCreateSetting) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_item(
        _setting: &ProcessItemCreateSetting,
        system_setting: &InitializeSystemAccessor,
    ) -> anyhow::Result<TProcessItemPtr> {
        let item = Self {
            common: ProcessControlItem::new(ProcessControlItemSetting {
                specifier: ENodeSpecifier::InternalDummy,
                systems: &system_setting,
            }),
        };
        Ok(SItemSPtr::new(item))
    }
}

impl TSystemCategory for DummyProcessData {}

impl TTimeTickCategory for DummyProcessData {
    fn can_support_offline() -> bool {
        true
    }

    fn can_support_realtime() -> bool {
        true
    }
}

impl TProcess for DummyProcessData {
    fn is_finished(&self) -> bool {
        true
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
        // 時間更新。またInputピンのリソース更新はしなくてもいい。
        self.common.elapsed_time = input.common.elapsed_time;
        self.common.process_input_pins();
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
