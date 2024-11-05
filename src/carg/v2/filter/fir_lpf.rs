use crate::carg::v2::meta::input::{EInputContainerCategoryFlag, EProcessInputContainer};
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::setting::Setting;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::{EProcessOutput, EProcessState, ProcessControlItem, ProcessOutputBuffer, ProcessProcessorInput, SItemSPtr, TProcess, TProcessItemPtr};
use crate::wave::sample::UniformedSample;
use crate::wave::PI2;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaFIRLPFInfo {
    /// エッジ周波数
    pub edge_frequency: f64,
    /// 遷移帯域幅の総周波数範囲
    pub delta_frequency: f64,
}

/// 内部変数の保持構造体
#[derive(Debug, Clone, Default)]
struct InternalInfo {
    /// 次のFIRLPF処理で入力バッファのスタート地点インデックス
    next_start_i: usize,
}

#[derive(Debug)]
pub struct FIRLPFProcessData {
    setting: Setting,
    common: ProcessControlItem,
    info: MetaFIRLPFInfo,
    internal: InternalInfo,
}

const INPUT_IN: &'static str = "in";
const OUTPUT_OUT: &'static str = "out";

impl FIRLPFProcessData {
    pub fn create_from(node: &ENode, setting: &Setting) -> TProcessItemPtr {
        if let ENode::FilterFIRLPF(v) = node {
            let item = Self {
                setting: setting.clone(),
                common: ProcessControlItem::new(ENodeSpecifier::FilterFIRLPF),
                info: v.clone(),
                internal: InternalInfo::default(),
            };
            return SItemSPtr::new(item);
        }
        unreachable!("Unexpected branch");
    }

    fn update_state(&mut self, in_input: &ProcessProcessorInput) {
        // まずLPFでは標本周波数が1として前提して計算を行うので、edgeとdeltaも変換する。
        let sample_rate = self.setting.sample_rate as f64;
        let edge = self.info.edge_frequency / sample_rate;
        let delta = self.info.delta_frequency / sample_rate;

        let can_process = self.update_input_buffer(in_input);
        if !can_process {
            return;
        }

        // フィルタ係数の数を計算する。
        // フィルタ係数の数は整数になるしかないし、またfilters_count+1が奇数じゃなきゃならない。
        // (Window Functionをちゃんと決めるため)
        let filters_count = compute_fir_lpf_filters_count(delta);
        let filter_responses = compute_fir_lpf_response(filters_count, edge);

        // また設定によってfilters_countが変わるので、ほかのノードのようにDrainすることはできない。
        // ただし今進行した分をIndexとして入れて、
        // そしてInputが大きくなったら処理に支障がない範囲で一番前をどんどんなくす。
        //
        // `filter_responses`を使って折り畳みを行う。
        let start_i = self.internal.next_start_i;

        let (buffer, setting) = {
            let item = self.common.get_input_internal(INPUT_IN).unwrap();
            let item = item.buffer_mono_dynamic().unwrap();
            let buffer = &item.buffer;
            let sample_range = start_i..buffer.len();

            let mut output_buffer = vec![];
            output_buffer.resize(sample_range.len(), UniformedSample::default());

            for sample_i in sample_range {
                for fc_i in 0..=filters_count {
                    if sample_i < fc_i {
                        break;
                    }

                    let output_i = sample_i - start_i;
                    output_buffer[output_i] += filter_responses[fc_i] * buffer[sample_i - fc_i];
                }
            }

            (output_buffer, item.setting.clone().unwrap())
        };

        // 処理が終わったら出力する。
        self.internal.next_start_i += buffer.len();
        self.common.insert_to_output_pin(
            OUTPUT_OUT,
            EProcessOutput::BufferMono(ProcessOutputBuffer::new(
                buffer,
                setting
            ))
        ).unwrap();

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
    fn update_input_buffer(&mut self, _in_input: &ProcessProcessorInput) -> bool {
        // 処理するためのバッファが十分じゃないと処理できない。
        let is_buffer_enough = match &*self.common.get_input_internal(INPUT_IN).unwrap() {
            EProcessInputContainer::BufferMonoDynamic(v) => {
                v.buffer.len() > 0
            }
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

impl TPinCategory for FIRLPFProcessData {
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

impl TProcess for FIRLPFProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn can_process(&self) -> bool { true }

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

/// FIRのLPFのフィルターカウント計算。
fn compute_fir_lpf_filters_count(delta: f64) -> usize {
    let mut filters_count = ((3.1 / delta).round() as isize) - 1;
    if (filters_count % 2) != 0 {
        filters_count += 1;
    }

    filters_count as usize
}

/// FIRのLPFの応答計算。
fn compute_fir_lpf_response(filters_count: usize, edge: f64) -> Vec<f64> {
    // isizeに変更する理由としては、responseを計算する際に負の数のIndexにも接近するため
    let filters_count = filters_count as isize;

    // -filters_count/2からfilters_count/2までにEWindowFunction(Hann)の値リストを求める。
    let windows = (0..=filters_count)
        .map(|v| {
            let sine = PI2 * ((v as f64) + 0.5) / ((filters_count + 1) as f64);
            (1.0 - sine) * 0.5
        })
        .collect_vec();

    // フィルタ係数の週はす特性bを計算する。
    let mut bs = (((filters_count >> 1) * -1)..=(filters_count >> 1))
        .map(|v| {
            let input = PI2 * edge * (v as f64);
            let sinc = if input == 0.0 { 1.0 } else { input.sin() / input };

            2.0 * edge * sinc
        })
        .collect_vec();

    assert_eq!(bs.len(), windows.len());
    for i in 0..windows.len() {
        bs[i] *= windows[i];
    }
    bs
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------