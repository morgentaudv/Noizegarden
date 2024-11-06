use crate::carg::v2::filter::iir_compute_sample;
use crate::carg::v2::meta::input::{EInputContainerCategoryFlag, EProcessInputContainer};
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::setting::Setting;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::{
    EProcessOutput, EProcessState, ProcessControlItem, ProcessOutputBuffer, ProcessProcessorInput, SItemSPtr, TProcess,
    TProcessItemPtr,
};
use crate::wave::sample::UniformedSample;
use crate::wave::PI2;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum EFilterMode {
    LowPass,
    HighPass,
    BandPass,
    BandRemove,
}

/// ノードの設定情報
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaIIRInfo {
    /// エッジ周波数（境界）
    pub edge_frequency: f64,
    /// 精密度
    pub quality_factor: f64,
}

/// 内部変数の保持構造体
#[derive(Debug, Clone)]
struct InternalInfo {
    /// 次のフィルタリング処理で入力バッファのスタート地点インデックス
    next_start_i: usize,
    /// モード
    mode: EFilterMode,
}

#[derive(Debug)]
pub struct IIRProcessData {
    setting: Setting,
    common: ProcessControlItem,
    info: MetaIIRInfo,
    internal: InternalInfo,
}

const INPUT_IN: &'static str = "in";
const OUTPUT_OUT: &'static str = "out";

impl TPinCategory for IIRProcessData {
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

impl TProcess for IIRProcessData {
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

impl IIRProcessData {
    pub fn create_from(node: &ENode, setting: &Setting, mode: EFilterMode) -> TProcessItemPtr {
        match node {
            ENode::FilterIIRLPF(v) => {
                let item = Self {
                    setting: setting.clone(),
                    common: ProcessControlItem::new(ENodeSpecifier::FilterIIRLPF),
                    info: v.clone(),
                    internal: InternalInfo { next_start_i: 0, mode },
                };

                SItemSPtr::new(item)
            }
            ENode::FilterIIRHPF(v) => {
                let item = Self {
                    setting: setting.clone(),
                    common: ProcessControlItem::new(ENodeSpecifier::FilterIIRHPF),
                    info: v.clone(),
                    internal: InternalInfo { next_start_i: 0, mode },
                };

                SItemSPtr::new(item)
            }
            _ => unreachable!("Unexpected branch"),
        }
    }

    fn update_state(&mut self, in_input: &ProcessProcessorInput) {
        let can_process = self.update_input_buffer();
        if !can_process {
            return;
        }

        // IIRの演算のための係数を計算する。
        let sample_rate = self.setting.sample_rate as f64;
        let (filter_as, filter_bs) = compute_filter_asbs(
            self.internal.mode,
            self.info.edge_frequency,
            sample_rate,
            self.info.quality_factor,
        );

        let (buffer, setting) = {
            let start_i = self.internal.next_start_i;
            let item = self.common.get_input_internal(INPUT_IN).unwrap();
            let item = item.buffer_mono_dynamic().unwrap();
            let buffer = &item.buffer;
            let sample_range = start_i..buffer.len();

            let mut output_buffer = vec![];
            output_buffer.resize(sample_range.len(), UniformedSample::default());

            for sample_i in sample_range {
                let output_i = sample_i - start_i;
                iir_compute_sample(output_i, sample_i, &mut output_buffer, buffer, &filter_as, &filter_bs);
            }

            (output_buffer, item.setting.clone().unwrap())
        };

        // 処理が終わったら出力する。
        self.internal.next_start_i += buffer.len();
        self.common
            .insert_to_output_pin(
                OUTPUT_OUT,
                EProcessOutput::BufferMono(ProcessOutputBuffer::new(buffer, setting)),
            )
            .unwrap();

        // 自分を終わるかしないかのチェック
        if in_input.is_children_all_finished() {
            self.common.state = EProcessState::Finished;
            return;
        } else {
            self.common.state = EProcessState::Playing;
            return;
        }
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

/// IIRのフィルタリングに使う遅延機フィルターの伝達関数の特性を計算する。
fn compute_filter_asbs(
    mode: EFilterMode,
    edge_frequency: f64,
    samples_per_sec: f64,
    quality_factor: f64,
) -> ([f64; 3], [f64; 3]) {
    match mode {
        EFilterMode::LowPass => {
            let analog_frequency = { 1.0 / PI2 * (edge_frequency * PI / samples_per_sec).tan() };
            let pi24a2 = 4.0 * PI.powi(2) * analog_frequency.powi(2);
            let pi2adivq = (PI2 * analog_frequency) / quality_factor;

            let b1 = pi24a2 / (1.0 + pi2adivq + pi24a2);
            let b2 = 2.0 * b1;
            let b3 = b1;
            let a1 = (2.0 * pi24a2 - 2.0) / (1.0 + pi2adivq + pi24a2);
            let a2 = (1.0 - pi2adivq + pi24a2) / (1.0 + pi2adivq + pi24a2);

            ([1.0, a1, a2], [b1, b2, b3])
        }
        EFilterMode::HighPass => {
            let analog_frequency = { 1.0 / PI2 * (edge_frequency * PI / samples_per_sec).tan() };
            // 4pi^2f_c^2
            let pi24a2 = 4.0 * PI.powi(2) * analog_frequency.powi(2);
            // 2pif_c / Q
            let pi2adivq = (PI2 * analog_frequency) / quality_factor;

            let b1 = 1.0 / (1.0 + pi2adivq + pi24a2);
            let b2 = -2.0 * b1;
            let b3 = b1;
            let a1 = (2.0 * pi24a2 - 2.0) * b1;
            let a2 = (1.0 - pi2adivq + pi24a2) * b1;

            ([1.0, a1, a2], [b1, b2, b3])
        }
        EFilterMode::BandPass => unimplemented!(),
        EFilterMode::BandRemove => unimplemented!(),
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
