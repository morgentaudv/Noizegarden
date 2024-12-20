use crate::carg::v2::filter::{iir_compute_sample, EFilterMode};
use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::setting::Setting;
use crate::carg::v2::meta::system::TSystemCategory;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::node::common::EProcessState;
use crate::carg::v2::{
    EProcessOutput, ProcessControlItem, ProcessOutputBuffer, ProcessProcessorInput, SItemSPtr, TProcess,
    TProcessItemPtr,
};
use crate::math::window::EWindowFunction;
use crate::wave::sample::UniformedSample;
use crate::wave::PI2;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use crate::carg::v2::meta::tick::TTimeTickCategory;

const SAMPLES: usize = 2048;
const OVERLAP_RATE: f64 = 0.5;

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
    /// 変形後、送る前のバッファ
    send_pending_buffer: Vec<UniformedSample>,
}

impl InternalInfo {
    fn new(mode: EFilterMode) -> Self {
        Self {
            next_start_i: 0,
            mode,
            send_pending_buffer: vec![],
        }
    }
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

impl TSystemCategory for IIRProcessData {}

impl TTimeTickCategory for IIRProcessData {
    fn can_support_offline() -> bool {
        true
    }

    fn can_support_realtime() -> bool {
        true
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
                    internal: InternalInfo::new(mode),
                };

                SItemSPtr::new(item)
            }
            ENode::FilterIIRHPF(v) => {
                let item = Self {
                    setting: setting.clone(),
                    common: ProcessControlItem::new(ENodeSpecifier::FilterIIRHPF),
                    info: v.clone(),
                    internal: InternalInfo::new(mode),
                };

                SItemSPtr::new(item)
            }
            ENode::FilterIIRBandPass(v) => {
                let item = Self {
                    setting: setting.clone(),
                    common: ProcessControlItem::new(ENodeSpecifier::FilterIIRBandPass),
                    info: v.clone(),
                    internal: InternalInfo::new(mode),
                };

                SItemSPtr::new(item)
            }
            ENode::FilterIIRBandStop(v) => {
                let item = Self {
                    setting: setting.clone(),
                    common: ProcessControlItem::new(ENodeSpecifier::FilterIIRBandStop),
                    info: v.clone(),
                    internal: InternalInfo::new(mode),
                };

                SItemSPtr::new(item)
            }
            _ => unreachable!("Unexpected branch"),
        }
    }

    fn update_state(&mut self, in_input: &ProcessProcessorInput) {
        let all_finished = in_input.is_children_all_finished();
        let (can_process, _required_samples) = self.update_input_buffer(all_finished);
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

        let (buffer, sample_rate) = {
            let item = self.common.get_input_internal(INPUT_IN).unwrap();
            let item = item.buffer_mono_dynamic().unwrap();
            let buffer = &item.buffer;

            let start_i = self.internal.next_start_i;
            let sample_range = start_i..(start_i + SAMPLES);

            let mut output_buffer = vec![];
            output_buffer.resize(SAMPLES, UniformedSample::default());

            for sample_i in sample_range {
                let output_i = sample_i - start_i;
                iir_compute_sample(output_i, sample_i, &mut output_buffer, buffer, &filter_as, &filter_bs);
            }

            let output_buffer = output_buffer
                .into_iter()
                .enumerate()
                .map(|(i, v)| EWindowFunction::Hann.get_factor_samples(i, SAMPLES) * v)
                .collect_vec();

            (output_buffer, item.sample_rate)
        };

        // 処理が終わったら出力する。
        // ただしOverlapしない部分だけ出力する。
        // Overlapされる部分は内部バッファに保持して、あとで次に処理したバッファと合わせる。
        let overlapped_len = (SAMPLES as f64 * OVERLAP_RATE) as usize;
        self.internal.next_start_i += overlapped_len;

        // 1. まずsend_pending_bufferとかけ合わせる。
        // pending_bufferの長さまではbufferに足して、余った分は後ろに追加する。
        let old_len = self.internal.send_pending_buffer.len();
        for add_i in 0..old_len {
            // Phase相殺は大丈夫か、これ？
            self.internal.send_pending_buffer[add_i] += buffer[add_i];
        }
        for push_i in old_len..buffer.len() {
            self.internal.send_pending_buffer.push(buffer[push_i]);
        }

        // 2. これ以上オーバーラップしないバッファだけをとって、次に送る。
        let un_overlapped_len = SAMPLES - overlapped_len;
        let send_buffer = self.internal.send_pending_buffer.drain(..un_overlapped_len).collect_vec();
        self.common
            .insert_to_output_pin(
                OUTPUT_OUT,
                EProcessOutput::BufferMono(ProcessOutputBuffer::new(send_buffer, sample_rate)),
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
    fn update_input_buffer(&mut self, all_children_finished: bool) -> (bool, usize) {
        // 処理するためのバッファが十分じゃないと処理できない。
        let mut item = self.common.get_input_internal_mut(INPUT_IN).unwrap();
        let item = &mut item.buffer_mono_dynamic_mut().unwrap();

        let required_samples = self.internal.next_start_i + SAMPLES;
        if item.buffer.len() < required_samples {
            return if all_children_finished {
                // すべての上からの処理が終わったら、新規のバッファは入ってこないはずなので
                // のこりの分を返さなきゃならない。
                // ただしサンプル数は最大限にして返す。0埋めした方がいいので。
                let offset = required_samples - item.buffer.len();
                for _ in 0..offset {
                    item.buffer.push(UniformedSample::default());
                }

                (true, SAMPLES)
            } else {
                (false, 0)
            }
        }

        // もしバッファが十分大きくなって、またインデックスも十分進んだら
        // 前に少し余裕分を残して削除する。
        if self.internal.next_start_i >= SAMPLES {
            // 前を削除する。
            let offset = SAMPLES.min(64);
            let drain_count = self.internal.next_start_i - offset;
            item.buffer.drain(..drain_count);
            self.internal.next_start_i = offset;
        }

        // 処理可能。
        (true, SAMPLES)
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
        EFilterMode::BandPass => {
            let analog_frequency = { 1.0 / PI2 * (edge_frequency * PI / samples_per_sec).tan() };
            // 4pi^2f_c^2
            let pi24a2 = 4.0 * PI.powi(2) * analog_frequency.powi(2);
            // 2pif_c / Q
            let pi2adivq = (PI2 * analog_frequency) / quality_factor;
            let div_base = 1.0 + pi2adivq + pi24a2;

            let b1 = pi2adivq / div_base;
            let b2 = 0.0;
            let b3 = b1 * -1.0;
            let a1 = (2.0 * pi24a2 - 2.0) / div_base;
            let a2 = (1.0 - pi2adivq + pi24a2) / div_base;

            ([1.0, a1, a2], [b1, b2, b3])
        }
        EFilterMode::BandStop => {
            let analog_frequency = { 1.0 / PI2 * (edge_frequency * PI / samples_per_sec).tan() };
            // 4pi^2f_c^2
            let pi24a2 = 4.0 * PI.powi(2) * analog_frequency.powi(2);
            // 2pif_c / Q
            let pi2adivq = (PI2 * analog_frequency) / quality_factor;
            let div_base = 1.0 + pi2adivq + pi24a2;

            let b1 = (pi24a2 + 1.0) / div_base;
            let b2 = (2.0 * pi24a2 - 2.0) / div_base;
            let b3 = b1;
            let a1 = b2;
            let a2 = (1.0 - pi2adivq + pi24a2) / div_base;

            ([1.0, a1, a2], [b1, b2, b3])
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
