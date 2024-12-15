use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::{ENode, SItemSPtr, Setting, TProcessItemPtr};
use crate::{
    carg::v2::{
        ProcessControlItem, ProcessOutputBuffer,
        ProcessProcessorInput, TProcess,
    },
    wave::sample::UniformedSample,
};
use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::output::EProcessOutputContainer;
use crate::carg::v2::meta::system::TSystemCategory;
use crate::carg::v2::node::common::EProcessState;

#[derive(Debug)]
pub struct AdapterEnvelopeAdsrProcessData {
    common: ProcessControlItem,
    /// 処理後に出力情報が保存されるところ。
    output: Option<ProcessOutputBuffer>,
    attack_time: f64,
    decay_time: f64,
    sustain_time: f64,
    release_time: f64,
    attack_curve: f64,
    decay_curve: f64,
    release_curve: f64,
    /// sustainで維持する振幅`[0, 1]`の値。
    sustain_value: f64,
}

impl TPinCategory for AdapterEnvelopeAdsrProcessData {
    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの入力側のピンの名前を返す。
    fn get_input_pin_names() -> Vec<&'static str> { vec!["in"] }

    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの出力側のピンの名前を返す。
    fn get_output_pin_names() -> Vec<&'static str> { vec!["out"] }

    /// 関係ノードに書いているピンのカテゴリ（複数可）を返す。
    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            "in" => Some(pin_category::BUFFER_MONO),
            "out" => Some(pin_category::BUFFER_MONO),
            _ => None,
        }
    }

    /// Inputピンのコンテナフラグ
    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            "in" => Some(input::container_category::BUFFER_MONO_PHANTOM),
            _ => None,
        }
    }
}

impl AdapterEnvelopeAdsrProcessData {
    pub fn create_from(node: &ENode, _setting: &Setting) -> TProcessItemPtr {
        match node {
            ENode::AdapterEnvelopeAdsr {
                attack_time,
                decay_time,
                sustain_time,
                release_time,
                attack_curve,
                decay_curve,
                release_curve,
                sustain_value,
            } => {
                let item = Self::new(
                    *attack_time,
                    *decay_time,
                    *sustain_time,
                    *release_time,
                    *attack_curve,
                    *decay_curve,
                    *release_curve,
                    *sustain_value);
                SItemSPtr::new(item)
            }
            _ => unreachable!("Unexpected branch."),
        }
    }

    pub fn new(
        attack_time: f64,
        decay_time: f64,
        sustain_time: f64,
        release_time: f64,
        attack_curve: f64,
        decay_curve: f64,
        release_curve: f64,
        sustain_value: f64,
    ) -> Self {
        assert!(attack_time >= 0.0);
        assert!(decay_time >= 0.0);
        assert!(attack_curve > 0.0);
        assert!(decay_curve > 0.0);

        Self {
            common: ProcessControlItem::new(ENodeSpecifier::AdapterEnvelopeAdsr),
            output: None,
            attack_time,
            decay_time,
            attack_curve,
            decay_curve,
            sustain_time,
            release_time,
            release_curve,
            sustain_value,
        }
    }
}

impl AdapterEnvelopeAdsrProcessData {
    fn update_state(&mut self, in_input: &ProcessProcessorInput) {
        // Inputがなきゃ何もできぬ。
        // これなに…
        let linked_output_pin = self
            .common
            .get_input_pin("in")
            .unwrap()
            .upgrade()
            .unwrap()
            .borrow()
            .linked_pins
            .first()
            .unwrap()
            .upgrade()
            .unwrap();

        let borrowed = linked_output_pin.borrow();
        let input = match &borrowed.output {
            EProcessOutputContainer::BufferMono(v) => v,
            _ => unreachable!("Unexpected branch"),
        };

        // このノードでは最初からADを行う。
        // もし尺が足りなければ、そのまま終わる。
        // inputのSettingのsample_rateから各バッファのサンプルの発生時間を計算する。
        let sample_rate = input.setting.sample_rate as f64;

        let decay_start_time = self.attack_time;
        let sustain_start_time = decay_start_time + self.decay_time;
        let release_start_time = sustain_start_time + self.sustain_time;
        let stop_time = release_start_time + self.release_time;

        let mut applied_buffer = vec![];
        applied_buffer.reserve(input.buffer.len());

        for (sample_i, sample) in input.buffer.iter().enumerate() {
            let sample_time = sample_i as f64 / sample_rate;

            if sample_time >= stop_time {
                // Envelopeが完全にとまったので。
                applied_buffer.push(UniformedSample::MIN);
            } else if sample_time >= release_start_time {
                // Release中。
                // curve < 1.0ならLog式、curve > 1.0なら指数関数式。
                let rate = (sample_time - release_start_time) / self.release_time;
                let input_rate = 1.0 - rate;
                // y = input_rate^(curve)。
                let value = self.sustain_value * input_rate.powf(self.release_curve);
                applied_buffer.push(value * *sample);
            } else if sample_time >= sustain_start_time {
                // Sustain中。
                applied_buffer.push(self.sustain_value * *sample);
            } else if sample_time >= decay_start_time {
                // Decay中。
                // curve < 1.0ならLog式、curve > 1.0なら指数関数式。
                let rate = (sample_time - decay_start_time) / self.decay_time;
                let input_rate = 1.0 - rate;
                // y = input_rate^(curve)。
                let value = (1.0 - self.sustain_value) * input_rate.powf(self.decay_curve) + self.sustain_value;
                applied_buffer.push(value * *sample);
            } else {
                // Attack中。
                // curve < 1.0ならLog式、curve > 1.0なら指数関数式。
                let rate = sample_time / self.attack_time;
                // y = input_rate^(curve)。
                let value = rate.powf(self.attack_curve);
                applied_buffer.push(value * *sample);
            }
        }

        // outputのどこかに保持する。
        self.output = Some(ProcessOutputBuffer::new(applied_buffer, input.setting.clone()));

        // 状態変更。
        if in_input.is_children_all_finished() {
            self.common.state = EProcessState::Finished;
            return;
        } else {
            self.common.state = EProcessState::Playing;
            return;
        }
    }
}

impl TSystemCategory for AdapterEnvelopeAdsrProcessData {}

impl TProcess for AdapterEnvelopeAdsrProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn can_process(&self) -> bool {
        self.common.is_all_input_pins_update_notified()
    }

    /// 共用アイテムの参照を返す。
    fn get_common_ref(&self) -> &ProcessControlItem {
        &self.common
    }

    /// 共用アイテムの可変参照を返す。
    fn get_common_mut(&mut self) -> &mut ProcessControlItem {
        &mut self.common
    }

    fn try_process(&mut self, input: &ProcessProcessorInput) {
        self.common.elapsed_time = input.common.elapsed_time;
        self.common.process_input_pins();

        match self.common.state {
            EProcessState::Stopped | EProcessState::Playing => self.update_state(input),
            _ => (),
        }
    }
}
