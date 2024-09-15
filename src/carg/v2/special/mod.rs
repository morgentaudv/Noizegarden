use crate::carg::v2::{ENode, EProcessState, ProcessControlItem, ProcessProcessorInput, SItemSPtr, Setting, TProcess, TProcessItemPtr};
use crate::carg::v2::meta::ENodeSpecifier;

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

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------


