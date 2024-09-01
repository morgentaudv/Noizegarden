use num_traits::Pow;

use crate::{
    carg::v2::{
        EProcessOutput, EProcessResult, EProcessState, ProcessControlItem, ProcessOutputBuffer,
        TInputBufferOutputBuffer,
    },
    wave::sample::UniformedSample,
};

#[derive(Debug)]
pub struct AdapterEnvelopeAdProcessData {
    common: ProcessControlItem,
    /// これじゃ一つしか受け入れない。
    input: Option<(usize, ProcessOutputBuffer)>,
    /// 処理後に出力情報が保存されるところ。
    output: Option<ProcessOutputBuffer>,
    attack_time: f64,
    decay_time: f64,
    attack_curve: f64,
    decay_curve: f64,
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
            attack_time,
            decay_time,
            attack_curve,
            decay_curve,
        }
    }
}

impl TInputBufferOutputBuffer for AdapterEnvelopeAdProcessData {
    fn set_child_count(&mut self, count: usize) {
        if count > 1 {
            println!("adapter-envelope-ad should have only one inputn node.");
        }

        self.common.child_count = count;
    }

    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn get_timestamp(&self) -> i64 {
        self.common.process_timestamp
    }

    fn get_output(&self) -> ProcessOutputBuffer {
        assert!(self.output.is_some());
        self.output.as_ref().unwrap().clone()
    }

    fn update_input(&mut self, index: usize, output: EProcessOutput) {
        if self.input.is_some() {
            let old_input = self.input.as_ref().unwrap().0;
            println!("adapter-envelope-ad node already has input information of ({}).", old_input);
        }

        match output {
            EProcessOutput::None => unimplemented!("Unexpected branch."),
            EProcessOutput::Buffer(v) => {
                self.input = Some((index, v));
            }
        }
    }

    fn try_process(&mut self) -> EProcessResult {
        if self.common.state == EProcessState::Finished {
            return EProcessResult::Finished;
        }

        // Inputがなきゃ何もできぬ。
        if self.input.is_none() {
            return EProcessResult::Pending;
        }

        // このノードでは最初からADを行う。
        // もし尺が足りなければ、そのまま終わる。
        // inputのSettingのsample_rateから各バッファのサンプルの発生時間を計算する。
        let (_, input) = self.input.as_ref().unwrap();
        let sample_rate = input.setting.sample_rate as f64;

        let stop_time = self.decay_time + self.attack_time;
        let decay_start_time = self.attack_time;

        let mut applied_buffer = vec![];
        applied_buffer.reserve(input.buffer.len());

        for (sample_i, sample) in input.buffer.iter().enumerate() {
            let sample_time = sample_i as f64 / sample_rate;

            if sample_time >= stop_time {
                // Envelopeが完全にとまったので。
                applied_buffer.push(UniformedSample::MIN);
            } else if sample_time >= decay_start_time {
                // Decay中。
                // curve < 1.0ならLog式、curve > 1.0なら指数関数式。
                let rate = (sample_time - decay_start_time) / self.decay_time;
                let input_rate = 1.0 - rate;
                // y = input_rate^(curve)。
                let value = input_rate.pow(self.decay_curve);
                applied_buffer.push(value * *sample);
            } else {
                // Attack中。
                // curve < 1.0ならLog式、curve > 1.0なら指数関数式。
                let rate = sample_time / self.attack_time;
                // y = input_rate^(curve)。
                let value = rate.pow(self.attack_curve);
                applied_buffer.push(value * *sample);
            }
        }

        // outputのどこかに保持する。
        self.output = Some(ProcessOutputBuffer {
            buffer: applied_buffer,
            setting: input.setting.clone(),
            range: input.range,
        });

        // 状態変更。
        self.common.state = EProcessState::Finished;
        self.common.process_timestamp += 1;
        return EProcessResult::Finished;
    }
}
