use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::output::EProcessOutputContainer;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::{
    EProcessOutput, EProcessState, EmitterRange, ProcessControlItem, ProcessOutputBuffer, ProcessProcessorInput,
    SItemSPtr, Setting, TProcess, TProcessItemPtr,
};
use crate::wave::analyze::method::ETransformMethod;
use crate::wave::analyze::transformer::{EExportSampleCountMode, FrequencyTransformer};

/// 周波数情報をもとに音波バッファを生成する。
#[derive(Debug)]
pub struct IDFTEmitterProcessData {
    setting: Setting,
    common: ProcessControlItem,
    sample_length: usize,
}

impl TPinCategory for IDFTEmitterProcessData {
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
            "in" => Some(pin_category::FREQUENCY),
            "out" => Some(pin_category::BUFFER_MONO),
            _ => None,
        }
    }

    /// Inputピンのコンテナフラグ
    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            "in" => Some(input::container_category::FREQUENCY_PHANTOM),
            _ => None,
        }
    }
}

impl IDFTEmitterProcessData {
    pub fn create_from(node: &ENode, setting: &Setting) -> TProcessItemPtr {
        match node {
            ENode::EmitterIDFT { sample_length } => {
                let item = IDFTEmitterProcessData {
                    setting: setting.clone(),
                    common: ProcessControlItem::new(ENodeSpecifier::EmitterIDFT),
                    sample_length: *sample_length,
                };
                SItemSPtr::new(item)
            }
            _ => unreachable!("Unexpected branch."),
        }
    }

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
            EProcessOutputContainer::Frequency(v) => v,
            _ => unreachable!("Unexpected branch"),
        };

        // IDFTで音がちゃんと合成できるかを確認する。
        let buffer = FrequencyTransformer {
            transform_method: ETransformMethod::IDFT,
            sample_count_mode: EExportSampleCountMode::Fixed(self.sample_length),
        }
        .transform_frequencies(&input.frequencies)
        .unwrap();

        // outputのどこかに保持する。
        self.common
            .insert_to_output_pin(
                "out",
                EProcessOutput::BufferMono(ProcessOutputBuffer {
                    buffer,
                    setting: self.setting.clone(),
                    range: EmitterRange {
                        start: 0.0,
                        length: 0.0,
                    }, // これいらなくね
                }),
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

impl TProcess for IDFTEmitterProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    /// 自分が処理可能なノードなのかを確認する。
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
        // 時間更新。またInputピンのリソース更新はしなくてもいい。
        self.common.elapsed_time = input.common.elapsed_time;
        self.common.process_input_pins();

        match self.common.state {
            EProcessState::Stopped | EProcessState::Playing => self.update_state(input),
            _ => (),
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
