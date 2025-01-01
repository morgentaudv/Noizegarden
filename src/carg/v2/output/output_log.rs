use crate::carg::v2::meta::input::{
    BufferMonoDynamicItem, EInputContainerCategoryFlag, EProcessInputContainer, TextDynamicItem,
};
use crate::carg::v2::meta::output::EProcessOutputContainer;
use crate::carg::v2::meta::system::{InitializeSystemAccessor, TSystemCategory};
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::node::common::{EProcessState, ProcessControlItemSetting};
use crate::carg::v2::{
    ENode, EParsedOutputLogMode, ProcessControlItem, ProcessItemCreateSetting, ProcessProcessorInput, SItemSPtr
    , TProcess, TProcessItem, TProcessItemPtr,
};

#[derive(Debug)]
pub struct OutputLogProcessData {
    common: ProcessControlItem,
    mode: EParsedOutputLogMode,
}

impl TProcessItem for OutputLogProcessData {
    fn can_create_item(_setting: &ProcessItemCreateSetting) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_item(
        setting: &ProcessItemCreateSetting,
        system_setting: &InitializeSystemAccessor,
    ) -> anyhow::Result<TProcessItemPtr> {
        Ok(match setting.node {
            ENode::OutputLog { mode } => {
                let item = Self {
                    common: ProcessControlItem::new(ProcessControlItemSetting {
                        specifier: ENodeSpecifier::OutputLog,
                        systems: &system_setting,
                    }),
                    mode: *mode,
                };

                SItemSPtr::new(item)
            }

            _ => unreachable!("Unexpected branch."),
        })
    }
}

impl TPinCategory for OutputLogProcessData {
    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの入力側のピンの名前を返す。
    fn get_input_pin_names() -> Vec<&'static str> {
        vec!["in"]
    }

    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの出力側のピンの名前を返す。
    fn get_output_pin_names() -> Vec<&'static str> {
        vec![]
    }

    /// 関係ノードに書いているピンのカテゴリ（複数可）を返す。
    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            "in" => Some(pin_category::BUFFER_MONO | pin_category::TEXT),
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

impl TSystemCategory for OutputLogProcessData {}

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

// ----------------------------------------------------------------------------
// EOutputLogItem
// ----------------------------------------------------------------------------

/// [`EProcessInputContainer::OutputLog`]の内部コンテナ
#[derive(Debug, Clone)]
pub enum EOutputLogItem {
    BuffersDynamic(BufferMonoDynamicItem),
    TextDynamic(TextDynamicItem),
}

impl EOutputLogItem {
    /// 今のセッティングで`output`が受け取れるか？
    pub fn can_support(&self, output: &EProcessOutputContainer) -> bool {
        match self {
            EOutputLogItem::BuffersDynamic(_) => match output {
                EProcessOutputContainer::Empty | EProcessOutputContainer::BufferMono(_) => true,
                _ => false,
            },
            EOutputLogItem::TextDynamic(_) => match output {
                EProcessOutputContainer::Empty | EProcessOutputContainer::Text(_) => true,
                _ => false,
            },
        }
    }

    /// `output`からセッティングをリセットする。
    pub fn reset_with(&mut self, output: &EProcessOutputContainer) {
        if self.can_support(output) {
            return;
        }

        match output {
            EProcessOutputContainer::BufferMono(_) => {
                *self = Self::BuffersDynamic(BufferMonoDynamicItem::new());
            }
            EProcessOutputContainer::Text(_) => {
                *self = Self::TextDynamic(TextDynamicItem::new());
            }
            _ => unreachable!("Unexpected branch"),
        }
    }

    /// 種類をかえずに中身だけをリセットする。
    pub fn reset(&mut self) {
        match self {
            EOutputLogItem::BuffersDynamic(v) => {
                *v = BufferMonoDynamicItem::new();
            }
            EOutputLogItem::TextDynamic(v) => {
                v.buffer.clear();
            }
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
