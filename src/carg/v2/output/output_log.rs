use crate::carg::v2::meta::input::{EInputContainerCategoryFlag, EProcessInputContainer};
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
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

impl TPinCategory for OutputLogProcessData {
    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの入力側のピンの名前を返す。
    fn get_input_pin_names() -> Vec<&'static str> { vec!["in"] }

    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの出力側のピンの名前を返す。
    fn get_output_pin_names() -> Vec<&'static str> { vec![] }

    /// 関係ノードに書いているピンのカテゴリ（複数可）を返す。
    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            "in" => Some(pin_category::WAVE_BUFFER | pin_category::TEXT),
            _ => None,
        }
    }

    /// Inputピンのコンテナフラグ
    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            "in" => Some(input::container_category::OUTPUT_LOG),
            _ => None,
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
