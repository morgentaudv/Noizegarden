use std::collections::HashMap;

use crate::carg::v2::{
    EParsedOutputLogMode, EProcessOutput, EProcessResult, EProcessState, ProcessControlItem, ProcessOutputBuffer,
    TInputBufferOutputNone, TProcess,
};

#[derive(Debug)]
pub struct OutputLogProcessData {
    common: ProcessControlItem,
    mode: EParsedOutputLogMode,
    inputs: HashMap<usize, ProcessOutputBuffer>,
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

impl TInputBufferOutputNone for OutputLogProcessData {
    fn get_timestamp(&self) -> i64 {
        self.common.process_timestamp
    }

    fn update_input(&mut self, index: usize, output: EProcessOutput) {
        match output {
            EProcessOutput::None => unimplemented!("Unexpected branch."),
            EProcessOutput::Buffer(v) => {
                self.inputs.insert(index, v);
            }
        }
    }

    fn set_child_count(&mut self, count: usize) {
        self.common.child_count = count;
    }
}

impl TProcess for OutputLogProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn try_process(&mut self, input: &crate::carg::v2::ProcessInput) -> EProcessResult {
        if self.common.child_count == 0 {
            self.common.state = EProcessState::Finished;
            self.common.process_timestamp += 1;
            return EProcessResult::Finished;
        }

        // Childrenが全部送信完了したら処理が行える。
        // commonで初期Childrenの数を比較するだけでいいかも。
        if self.inputs.len() < self.common.child_count {
            return EProcessResult::Pending;
        }
        assert!(self.common.child_count > 0);

        // 出力する。
        match self.mode {
            EParsedOutputLogMode::Print => {
                for (i, input) in &self.inputs {
                    println!("Index : {i}");
                    println!("{:?}", input);
                    println!("");
                }
            }
        }

        self.common.state = EProcessState::Finished;
        self.common.process_timestamp += 1;
        return EProcessResult::Finished;
    }
}
