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
    inputs: HashMap<usize, ChildInputInfo>,
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
    fn update_state_stopped(&mut self, input: &ProcessProcessorInput) -> EProcessResult {
        // 出力する。
        match self.mode {
            EParsedOutputLogMode::Print => {
                for (i, input) in &mut self.inputs {
                    let logs = input.drain_buffer_if_updated();
                    if logs.is_empty() {
                        continue;
                    }

                    println!("Index : {i}");
                    logs.into_iter().for_each(|v| println!("{:?}", v));
                    println!("");
                }
            }
        }

        // その後にFinishしたら終わる。
        let is_children_finished = input.children_states.iter().all(|v| *v == EProcessState::Finished);
        if self.inputs.len() >= self.common.child_count && is_children_finished {
            self.common.state = EProcessState::Finished;
            self.common.process_timestamp += 1;
            return EProcessResult::Finished;
        }

        // じゃなきゃPlayingに。
        self.common.state = EProcessState::Playing;
        self.common.process_timestamp += 1;
        return EProcessResult::Pending;
    }
}

impl TInputBufferOutputNone for OutputLogProcessData {
    fn get_timestamp(&self) -> i64 {
        self.common.process_timestamp
    }

    fn set_child_count(&mut self, count: usize) {
        self.common.child_count = count;
    }

    fn update_input(&mut self, index: usize, output: EProcessOutput) {
        match output {
            EProcessOutput::None => unimplemented!("Unexpected branch."),
            EProcessOutput::Buffer(v) => {
                if !self.inputs.contains_key(&index) {
                    self.inputs.insert(index, ChildInputInfo::new());
                }

                self.inputs.get_mut(&index).unwrap().insert_buffer(v);
            }
        }
    }
}

impl TProcess for OutputLogProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn get_state(&self) -> EProcessState {
        self.common.state
    }

    fn try_process(&mut self, input: &ProcessProcessorInput) -> EProcessResult {
        match self.common.state {
            EProcessState::Stopped | EProcessState::Playing => self.update_state_stopped(input),
            EProcessState::Finished => {
                return EProcessResult::Finished;
            }
        }
    }
}
