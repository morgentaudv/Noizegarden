use itertools::Itertools;
use num_traits::Pow;

use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::output::EProcessOutputContainer;
use crate::carg::v2::meta::system::{InitializeSystemAccessor, TSystemCategory};
use crate::carg::v2::meta::tick::TTimeTickCategory;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::node::common::{EProcessState, ProcessControlItemSetting};
use crate::carg::v2::{
    ENode, EProcessOutput, ProcessControlItem, ProcessItemCreateSetting, ProcessOutputBuffer, ProcessProcessorInput,
    SItemSPtr, TProcess, TProcessItem, TProcessItemPtr,
};

/// ユニット単位でADEnvelopeを生成するための時間に影響しないエミッタ。
#[derive(Debug, Clone)]
struct EnvelopeAdValueEmitter {
    attack_time: f64,
    decay_time: f64,
    attack_curve: f64,
    decay_curve: f64,
    next_sample_index: usize,
}

impl EnvelopeAdValueEmitter {
    fn new(attack_time: f64, decay_time: f64, attack_curve: f64, decay_curve: f64) -> Self {
        Self {
            attack_time,
            decay_time,
            attack_curve,
            decay_curve,
            next_sample_index: 0usize,
        }
    }

    pub fn next_value(&mut self, sample_rate: usize) -> f64 {
        let unit_time = self.next_sample_index as f64;
        self.next_sample_index += 1;

        let sample_time = unit_time / (sample_rate as f64);
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
    emitter: EnvelopeAdValueEmitter,
}

impl TPinCategory for AdapterEnvelopeAdProcessData {
    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの入力側のピンの名前を返す。
    fn get_input_pin_names() -> Vec<&'static str> {
        vec!["in"]
    }

    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの出力側のピンの名前を返す。
    fn get_output_pin_names() -> Vec<&'static str> {
        vec!["out"]
    }

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

impl TProcessItem for AdapterEnvelopeAdProcessData {
    fn can_create_item(_setting: &ProcessItemCreateSetting) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_item(
        setting: &ProcessItemCreateSetting,
        system_setting: &InitializeSystemAccessor,
    ) -> anyhow::Result<TProcessItemPtr> {
        match setting.node {
            ENode::AdapterEnvelopeAd {
                attack_time,
                decay_time,
                attack_curve,
                decay_curve,
            } => {
                assert!(*attack_time >= 0.0);
                assert!(*decay_time >= 0.0);
                assert!(*attack_curve > 0.0);
                assert!(*decay_curve > 0.0);

                let item =         Self {
                    common: ProcessControlItem::new(ProcessControlItemSetting {
                        specifier: ENodeSpecifier::AdapterEnvelopeAd,
                        systems: &system_setting,
                    }),
                    emitter: EnvelopeAdValueEmitter::new(*attack_time, *decay_time, *attack_curve, *decay_curve),
                };
                Ok(SItemSPtr::new(item))
            }
            _ => unreachable!("Unexpected branch."),
        }
    }
}

impl AdapterEnvelopeAdProcessData {
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
        let values = self.emitter.next_values(input.buffer.len(), input.sample_rate);
        let buffer = input.buffer.iter().zip(values.iter()).map(|(a, b)| *b * *a).collect_vec();

        // outputのどこかに保持する。
        self.common
            .insert_to_output_pin(
                "out",
                EProcessOutput::BufferMono(ProcessOutputBuffer::new(buffer, input.sample_rate)),
            )
            .unwrap();

        if in_input.is_children_all_finished() {
            self.common.state = EProcessState::Finished;
            return;
        } else {
            self.common.state = EProcessState::Playing;
            return;
        }
    }
}

impl TSystemCategory for AdapterEnvelopeAdProcessData {}

impl TTimeTickCategory for AdapterEnvelopeAdProcessData {
    fn can_support_offline() -> bool {
        true
    }

    fn can_support_realtime() -> bool {
        true
    }
}

impl TProcess for AdapterEnvelopeAdProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn can_process(&self) -> bool {
        true
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
        self.common.process_input_pins_deprecated();

        match self.common.state {
            EProcessState::Stopped | EProcessState::Playing => self.update_state(input),
            _ => (),
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
