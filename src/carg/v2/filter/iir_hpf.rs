use std::f64::consts::PI;
use serde::{Deserialize, Serialize};
use crate::carg::v2::meta::setting::Setting;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::meta::input::{EInputContainerCategoryFlag, EProcessInputContainer};
use crate::carg::v2::{EProcessOutput, EProcessState, ProcessControlItem, ProcessOutputBuffer, ProcessProcessorInput, SItemSPtr, TProcess, TProcessItemPtr};
use crate::carg::v2::filter::iir_compute_sample;
use crate::carg::v2::meta::node::ENode;
use crate::wave::PI2;
use crate::wave::sample::UniformedSample;

/// ノードの設定情報
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaIIRHPFInfo {
    /// エッジ周波数（境界）
    pub edge_frequency: f64,
    /// 精密度
    pub quality_factor: f64,
}

/// 内部変数の保持構造体
#[derive(Debug, Clone, Default)]
struct InternalInfo {
    /// 次のフィルタリング処理で入力バッファのスタート地点インデックス
    next_start_i: usize,
}

#[derive(Debug)]
pub struct IIRHPFProcessData {
    setting: Setting,
    common: ProcessControlItem,
    info: MetaIIRHPFInfo,
    internal: InternalInfo,
}

const INPUT_IN: &'static str = "in";
const OUTPUT_OUT: &'static str = "out";

impl TPinCategory for IIRHPFProcessData {
    fn get_input_pin_names() -> Vec<&'static str> {
        vec![INPUT_IN]
    }

    fn get_output_pin_names() -> Vec<&'static str> {
        vec![OUTPUT_OUT]
    }

    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(pin_category::BUFFER_MONO),
            OUTPUT_OUT => Some(pin_category::BUFFER_MONO),
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

impl TProcess for IIRHPFProcessData {
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

impl IIRHPFProcessData {
    pub fn create_from(node: &ENode, setting: &Setting) -> TProcessItemPtr {
        if let ENode::FilterIIRHPF(v) = node {
            let item = Self {
                setting: setting.clone(),
                common: ProcessControlItem::new(ENodeSpecifier::FilterIIRLPF),
                info: v.clone(),
                internal: InternalInfo::default(),
            };
            return SItemSPtr::new(item);
        }
        unreachable!("Unexpected branch");
    }

    fn update_state(&mut self, in_input: &ProcessProcessorInput) {
    }

    /// Input側のバッファと内部処理の情報を更新し、またフィルタリングの処理が行えるかを判定する。
    fn update_input_buffer(&mut self) -> bool {
        // 処理するためのバッファが十分じゃないと処理できない。
        let is_buffer_enough = match &*self.common.get_input_internal(INPUT_IN).unwrap() {
            EProcessInputContainer::BufferMonoDynamic(v) => v.buffer.len() > 0,
            _ => false,
        };
        if !is_buffer_enough {
            return false;
        }

        // もしバッファが十分大きくなって、またインデックスも十分進んだら
        // 前に少し余裕分を残して削除する。
        let mut item = self.common.get_input_internal_mut(INPUT_IN).unwrap();
        let item = &mut item.buffer_mono_dynamic_mut().unwrap();
        if item.buffer.len() >= 4096 && self.internal.next_start_i >= 2048 {
            // 前を削除する。
            let drain_count = self.internal.next_start_i - 96;
            item.buffer.drain(..drain_count);
            self.internal.next_start_i = 96;
        }

        // 処理可能。
        true
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
