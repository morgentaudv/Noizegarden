use itertools::Itertools;

use crate::carg::v2::meta::input::EProcessInputContainer;
use crate::carg::v2::meta::ENodeSpecifier;
use crate::carg::v2::{
    ENode, EParsedOutputLogMode, EProcessOutput, EProcessState, ProcessControlItem, ProcessProcessorInput, SItemSPtr,
    Setting, TProcess, TProcessItemPtr,
};

/// [`OutputLogProcessData`]専用で
#[derive(Debug)]
struct ChildInputInfo {
    buffers: Vec<EProcessOutput>,
    is_new_inserted: bool,
}

impl ChildInputInfo {
    fn new() -> Self {
        Self {
            buffers: vec![],
            is_new_inserted: false,
        }
    }

    fn insert_buffer(&mut self, buffer: EProcessOutput) {
        self.buffers.push(buffer);
        self.is_new_inserted = true;
    }

    fn drain_buffer_if_updated(&mut self) -> Vec<String> {
        if !self.is_new_inserted {
            return vec![];
        }

        let logs = self.buffers.iter().map(|v| format!("{:?}", v)).collect_vec();
        self.buffers.clear();
        self.is_new_inserted = false;

        logs
    }
}

// ----------------------------------------------------------------------------
// OutputLogProcessData
// ----------------------------------------------------------------------------

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
