use itertools::Itertools;
use num_traits::Pow;

use crate::carg::v2::{
    EProcessOutput, EProcessResult, EProcessState, ProcessControlItem, ProcessOutputBuffer, ProcessProcessorInput,
    TInputBufferOutputBuffer, TProcess,
};

/// ユニット単位でADEnvelopeを生成するための時間に影響しないエミッタ。
#[derive(Debug, Clone)]
pub struct EnvelopeAdValueEmitter {
    attack_time: f64,
    decay_time: f64,
    attack_curve: f64,
    decay_curve: f64,
    next_sample_index: usize,
}

impl EnvelopeAdValueEmitter {
    pub fn new(attack_time: f64, decay_time: f64, attack_curve: f64, decay_curve: f64) -> Self {
        Self {
            attack_time,
            decay_time,
            attack_curve,
            decay_curve,
            next_sample_index: 0usize,
        }
    }

    pub fn next_value(&mut self, sample_rate: usize) -> f64 {
        let unittime = self.next_sample_index as f64;
        self.next_sample_index += 1;

        let sample_time = unittime / (sample_rate as f64);
        let stop_time = self.decay_time + self.attack_time;
        let decay_start_time = self.attack_time;

        if sample_time >= stop_time {
            // Envelopeが完全にとまったので。
            0.0
        } else if sample_time >= decay_start_time {
            // Decay中。
            // curve < 1.0ならLog式、curve > 1.0なら指数関数式。
            let rate = (sample_time - decay_start_time) / self.decay_time;
            let input_rate = 1.0 - rate;
            // y = input_rate^(curve)。
            input_rate.pow(self.decay_curve)
        } else {
            // Attack中。
            // curve < 1.0ならLog式、curve > 1.0なら指数関数式。
            let rate = sample_time / self.attack_time;
            // y = input_rate^(curve)。
            rate.pow(self.attack_curve)
        }
    }

    /// `length`分の値を取得する。
    pub fn next_values(&mut self, length: usize, sample_rate: usize) -> Vec<f64> {
        if length == 0 {
            vec![]
        } else {
            (0..length).map(|_| self.next_value(sample_rate)).collect_vec()
        }
    }
}

// ----------------------------------------------------------------------------
// AdapterEnvelopeAdProcessData
// ----------------------------------------------------------------------------

#[derive(Debug)]
pub struct AdapterEnvelopeAdProcessData {
    common: ProcessControlItem,
    /// これじゃ一つしか受け入れない。
    input: Option<(String, ProcessOutputBuffer)>,
    /// 処理後に出力情報が保存されるところ。
    output: Option<ProcessOutputBuffer>,
    emitter: EnvelopeAdValueEmitter,
}

impl AdapterEnvelopeAdProcessData {
    pub fn new(attack_time: f64, decay_time: f64, attack_curve: f64, decay_curve: f64) -> Self {
        assert!(attack_time >= 0.0);
        assert!(decay_time >= 0.0);
        assert!(attack_curve > 0.0);
        assert!(decay_curve > 0.0);

        Self {
            common: ProcessControlItem::new(),
            input: None,
            output: None,
            emitter: EnvelopeAdValueEmitter::new(attack_time, decay_time, attack_curve, decay_curve),
        }
    }
}

impl AdapterEnvelopeAdProcessData {
    fn update_state(&mut self, in_input: &ProcessProcessorInput) -> EProcessResult {
        // Inputがなきゃ何もできぬ。
        if self.input.is_none() {
            return EProcessResult::Pending;
        }

        // このノードでは最初からADを行う。
        // もし尺が足りなければ、そのまま終わる。
        // inputのSettingのsample_rateから各バッファのサンプルの発生時間を計算する。
        let (_, input) = self.input.as_ref().unwrap();
        let values = self.emitter.next_values(input.buffer.len(), input.setting.sample_rate as usize);
        let buffer = input.buffer.iter().zip(values.iter()).map(|(a, b)| *b * *a).collect_vec();
        println!("{:?}", buffer);

        // outputのどこかに保持する。
        self.output = Some(ProcessOutputBuffer {
            buffer,
            setting: input.setting.clone(),
            range: input.range,
        });

        // 状態変更。
        self.common.process_timestamp += 1;

        if in_input.is_children_all_finished() {
            self.common.state = EProcessState::Finished;
            return EProcessResult::Finished;
        } else {
            self.common.state = EProcessState::Playing;
            return EProcessResult::Pending;
        }
    }
}

impl TInputBufferOutputBuffer for AdapterEnvelopeAdProcessData {
    fn get_output(&self) -> ProcessOutputBuffer {
        assert!(self.output.is_some());
        self.output.as_ref().unwrap().clone()
    }

    /// 自分のノードに[`input`]を入れるか判定して適切に処理する。
    fn update_input(&mut self, node_name: &str, input: &EProcessOutput) {
        match input {
            EProcessOutput::None => unimplemented!("Unexpected branch."),
            EProcessOutput::Buffer(v) => {
                self.input = Some((node_name.to_owned(), v.clone()));
            }
        }
    }
}

impl TProcess for AdapterEnvelopeAdProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn try_process(&mut self, input: &ProcessProcessorInput) -> EProcessResult {
        match self.common.state {
            EProcessState::Stopped | EProcessState::Playing => self.update_state(input),
            EProcessState::Finished => {
                return EProcessResult::Finished;
            }
        }
    }

    fn can_process(&self) -> bool {
        self.input.is_some()
    }
}
