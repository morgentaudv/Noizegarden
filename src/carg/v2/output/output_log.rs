use std::collections::HashMap;

use itertools::Itertools;

use crate::carg::v2::{
    EParsedOutputLogMode, EProcessOutput, EProcessResult, EProcessState, ProcessControlItem, ProcessOutputBuffer,
    ProcessProcessorInput, TInputBufferOutputNone, TProcess,
};

// ----------------------------------------------------------------------------
// ChildInputInfo
// ----------------------------------------------------------------------------

/// [`OutputLogProcessData`]専用で
#[derive(Debug)]
struct ChildInputInfo {
    buffers: Vec<ProcessOutputBuffer>,
    is_new_inserted: bool,
}

impl ChildInputInfo {
    fn new() -> Self {
        Self {
            buffers: vec![],
            is_new_inserted: false,
        }
    }

    fn insert_buffer(&mut self, buffer: ProcessOutputBuffer) {
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
    inputs: HashMap<String, ChildInputInfo>,
}

impl OutputLogProcessData {
    pub fn new(mode: EParsedOutputLogMode) -> Self {
        Self {
            common: ProcessControlItem::new(),
            mode,
            inputs: HashMap::new(),
        }
    }
}

impl OutputLogProcessData {
    fn update_state(&mut self, _input: &ProcessProcessorInput) -> EProcessResult {
        // 出力する。
        match self.mode {
            EParsedOutputLogMode::Print => {
                // 使ってから
                for (i, input) in &mut self.inputs {
                    let logs = input.drain_buffer_if_updated();
                    if logs.is_empty() {
                        continue;
                    }

                    println!("Index : {i}");
                    logs.into_iter().for_each(|v| println!("{:?}", v));
                    println!("");
                }

                // Drain。
                self.inputs.clear();
            }
        }

        // じゃなきゃPlayingに。
        self.common.state = EProcessState::Playing;
        self.common.process_timestamp += 1;
        return EProcessResult::Finished;
    }
}

impl TInputBufferOutputNone for OutputLogProcessData {
    /// 自分のノードに[`input`]を入れるか判定して適切に処理する。
    fn update_input(&mut self, node_name: &str, input: &EProcessOutput) {
        match input {
            EProcessOutput::None => unimplemented!("Unexpected branch."),
            EProcessOutput::Buffer(v) => {
                if !self.inputs.contains_key(node_name) {
                    self.inputs.insert(node_name.to_owned(), ChildInputInfo::new());
                }

                self.inputs.get_mut(node_name).unwrap().insert_buffer(v.clone());
            }
        }
    }
}

impl TProcess for OutputLogProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn try_process(&mut self, input: &ProcessProcessorInput) -> EProcessResult {
        self.update_state(input)
    }

    /// 自分は内部状態に関係なくいつでも処理できる。
    fn can_process(&self) -> bool {
        match self.mode {
            EParsedOutputLogMode::Print => true,
        }
    }
}
