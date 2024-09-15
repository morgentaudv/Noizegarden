use crate::carg::v2::meta::input::EProcessInputContainer;
use crate::carg::v2::meta::ENodeSpecifier;
use crate::carg::v2::{
    ENode, EParsedOutputLogMode, EProcessState, ProcessControlItem, ProcessProcessorInput, SItemSPtr,
    Setting, TProcess, TProcessItemPtr,
};

#[derive(Debug)]
pub struct OutputLogProcessData {
    common: ProcessControlItem,
    mode: EParsedOutputLogMode,
}

impl OutputLogProcessData {
    pub fn create_from(node: &ENode, _setting: &Setting) -> TProcessItemPtr {
        match node {
            ENode::OutputLog { mode } => {
                let item = Self::new(mode.clone());
                SItemSPtr::new(item)
            }
            _ => unreachable!("Unexpected branch."),
        }
    }

    fn new(mode: EParsedOutputLogMode) -> Self {
        Self {
            common: ProcessControlItem::new(ENodeSpecifier::OutputLog),
            mode,
        }
    }
}

impl OutputLogProcessData {
    fn update_state(&mut self, _input: &ProcessProcessorInput) {
        // 出力する。
        match self.mode {
            EParsedOutputLogMode::Print => {
                let string = match &mut self.common.input_pins.get("in").unwrap().borrow_mut().input {
                    EProcessInputContainer::OutputLog(v) => {
                        let string = format!("{:?}", v);
                        v.reset(); // Drain。
                        string
                    }
                    _ => unreachable!("Unexpected input."),
                };

                println!("{}", string);
                println!();
            }
        }

        // じゃなきゃPlayingに。
        self.common.state = EProcessState::Playing;
    }
}

impl TProcess for OutputLogProcessData {
    fn is_finished(&self) -> bool {
        true
    }

    /// 自分は内部状態に関係なくいつでも処理できる。
    fn can_process(&self) -> bool {
        match self.mode {
            EParsedOutputLogMode::Print => true,
        }
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

        self.update_state(input)
    }
}
