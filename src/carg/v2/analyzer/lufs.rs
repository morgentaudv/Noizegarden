use serde::{Deserialize, Serialize};
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::meta::input::{EInputContainerCategoryFlag, EProcessInputContainer};
use crate::carg::v2::{EProcessState, ProcessControlItem, ProcessProcessorInput, SItemSPtr, TProcess, TProcessItemPtr};
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::setting::Setting;

mod hz48000
{

    pub const C1: f64 = -1.99004745483398;
    pub const C2: f64 = 0.99007225036621;

    pub const IIR_AS: [f64; 3] = [1.0, -1.69065929318241, 0.73248077421585];
    pub const IIR_BS: [f64; 3] = [1.53512485958697, -2.69169618940638, 1.19839281065285];
    pub const IIR_CS: [f64; 3] = [1.0, C1, C2];
    pub const IIR_DS: [f64; 3] = [1.0, -2.0, 1.0];
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaLufsInfo {
    /// インプットとして入力されるサンプルバッファのサンプルレートを代わりに使うか否か
    pub use_input: bool,
    /// LUの測定で一ブロックを動かす周期秒。
    pub slide_length: f64,
    /// LUの想定の基本単位。
    pub block_length: f64,
}

#[derive(Debug)]
pub struct AnalyzeLUFSProcessData {
    setting: Setting,
    common: ProcessControlItem,
    info: MetaLufsInfo,
    internal: InternalInfo,
}

const INPUT_IN: &'static str = "in";
const OUTPUT_OUT: &'static str = "out";

/// 測定結果
#[derive(Default, Debug, Clone, Copy)]
struct LUFSResult {
    /// 測定開始時間
    start_second: f64,
    /// 長さ
    length: f64,
    /// LUFSのデシベルレベル
    db_value: f64,
}

/// 内部情報
#[derive(Default, Debug, Clone, Copy)]
struct InternalInfo {
    /// 次のフィルタリング処理で入力バッファのスタート地点インデックス
    next_start_i: usize,
    /// 処理した時間秒
    processed_time: f64,
}

impl TPinCategory for AnalyzeLUFSProcessData {
    fn get_input_pin_names() -> Vec<&'static str> {
        vec![INPUT_IN]
    }

    fn get_output_pin_names() -> Vec<&'static str> {
        vec![OUTPUT_OUT]
    }

    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(pin_category::BUFFER_MONO),
            OUTPUT_OUT => Some(pin_category::TEXT),
            _ => None,
        }
    }

    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(input::container_category::BUFFER_MONO_DYNAMIC),
            _ => None,
        }
    }
}

impl TProcess for AnalyzeLUFSProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn can_process(&self) -> bool {
        true
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

        match self.common.state {
            EProcessState::Stopped | EProcessState::Playing => self.update_state(input),
            _ => (),
        }
    }
}

impl AnalyzeLUFSProcessData {
    pub fn create_from(node: &ENode, setting: &Setting) -> TProcessItemPtr {
        if let ENode::AnalyzerLUFS(v) = node {
            let item= Self {
                setting: setting.clone(),
                common: ProcessControlItem::new(ENodeSpecifier::AnalyzerLUFS),
                info: v.clone(),
                internal: InternalInfo::default(),
            };

            return SItemSPtr::new(item);
        }

        unreachable!("Unexpected branch");
    }

    fn update_state(&mut self, in_input: &ProcessProcessorInput) {
        let can_process = self.update_input_buffer();
        if !can_process {
            return;
        }

        let sample_rate = if self.info.use_input {
            self.setting.sample_rate as f64
        }
        else {
            self.setting.sample_rate as f64
        };

        // `block_length`から処理ができるかを確認する。
        // もしインプットが終わったなら、尺が足りなくてもそのまま処理する。
        let slide_sample_len = (self.info.slide_length as f64 * sample_rate).floor() as usize;
        let block_sample_len = (self.info.block_length as f64 * sample_rate).ceil() as usize;


    }

    /// Input側のバッファと内部処理の情報を更新し、またフィルタリングの処理が行えるかを判定する。
    fn update_input_buffer(&mut self) -> bool {
        // `block_length`から処理ができるかを確認する。
        // もしインプットが終わったなら、尺が足りなくてもそのまま処理する。
        let sample_rate = if self.info.use_input {
            self.setting.sample_rate as f64
        }
        else {
            self.setting.sample_rate as f64
        };
        let slide_sample_len = (self.info.slide_length as f64 * sample_rate).floor() as usize;
        let block_sample_len = (self.info.block_length as f64 * sample_rate).ceil() as usize;

        // TODO
        // 1.

        // 処理するためのバッファが十分じゃないと処理できない。
        let is_buffer_enough = match &*self.common.get_input_internal(INPUT_IN).unwrap() {
            EProcessInputContainer::BufferMonoDynamic(v) => v.buffer.len() > 0,
            _ => false,
        };
        if !is_buffer_enough {
            return false;
        }



        return true;
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
