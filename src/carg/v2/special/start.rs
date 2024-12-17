use crate::carg::v2::{ENode, ProcessControlItem, ProcessProcessorInput, SItemSPtr, Setting, TProcess, TProcessItemPtr};
use crate::carg::v2::meta::{pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::system::TSystemCategory;
use crate::carg::v2::meta::tick::TTimeTickCategory;
use crate::carg::v2::node::common::EProcessState;

/// スタートノード
#[derive(Debug)]
pub struct StartProcessData
{
    common: ProcessControlItem,
}

impl StartProcessData
{
    pub fn create_from(_node: &ENode, _setting: &Setting) -> TProcessItemPtr
    {
        let item = Self
        {
            common: ProcessControlItem::new(ENodeSpecifier::InternalStartPin)
        };
        SItemSPtr::new(item)
    }
}

impl TSystemCategory for StartProcessData {}

impl TTimeTickCategory for StartProcessData {
    fn can_support_offline() -> bool {
        true
    }

    fn can_support_realtime() -> bool {
        true
    }
}

impl TPinCategory for StartProcessData {
    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの入力側のピンの名前を返す。
    fn get_input_pin_names() -> Vec<&'static str> { vec![] }

    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの出力側のピンの名前を返す。
    fn get_output_pin_names() -> Vec<&'static str> { vec!["out"] }

    /// 関係ノードに書いているピンのカテゴリ（複数可）を返す。
    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            "out" => Some(pin_category::START),
            _ => None,
        }
    }

    /// Inputピンのコンテナフラグ
    fn get_input_container_flag(_pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        None
    }
}

impl TProcess for StartProcessData
{
    fn is_finished(&self) -> bool { self.common.state == EProcessState::Finished }

    /// いつも更新できる。
    fn can_process(&self) -> bool { true }

    /// 共用アイテムの参照を返す。
    fn get_common_ref(&self) -> &ProcessControlItem { &self.common }

    /// 共用アイテムの可変参照を返す。
    fn get_common_mut(&mut self) -> &mut ProcessControlItem { &mut self.common }

    fn try_process(&mut self, input: &ProcessProcessorInput)
    {
        self.common.elapsed_time = input.common.elapsed_time;

        // いつも次のノードが処理が走れるようにする。
        for (_, pin) in &mut self.common.output_pins
        {
            pin.borrow_mut().elapsed_time = self.common.elapsed_time;
            pin.borrow_mut().notify_update_to_next_pins();
        }
    }
}
