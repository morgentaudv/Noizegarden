use itertools::Itertools;
use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::setting::Setting;
use crate::carg::v2::meta::system::{InitializeSystemAccessor, TSystemCategory};
use crate::carg::v2::meta::tick::TTimeTickCategory;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::node::common::{EProcessState, ProcessControlItem, ProcessControlItemSetting};
use crate::carg::v2::{
    EProcessOutput, ProcessItemCreateSetting, ProcessOutputBuffer, ProcessProcessorInput, SItemSPtr, TProcess,
    TProcessItem, TProcessItemPtr,
};
use crate::nz_define_time_tick_for;
use crate::wave::sample::UniformedSample;
use serde::{Deserialize, Serialize};
use soundprog::math::get_required_sample_count;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaDelayInfo {
    /// Delayする秒数。マイナスの値は禁止
    pub delay: f64,
}

#[derive(Debug)]
pub struct AdapterDelayProcessData {
    setting: Setting,
    common: ProcessControlItem,
    internal: InternalInfo,
}

#[derive(Debug)]
struct InternalInfo {
    /// 設定情報
    info: MetaDelayInfo,
    /// [`MetaDelayInfo::delay`]分マイナスから始める。
    internal_time: f64,
    /// サンプルを取得するための最後に処理した時間
    last_process_time: f64,
}

impl InternalInfo {
    fn new(info: MetaDelayInfo) -> Self {
        assert!(info.delay >= 0.0);

        let delay = info.delay * -1.0;
        Self {
            info,
            internal_time: delay,
            last_process_time: delay,
        }
    }
}

const INPUT_IN: &'static str = "in";
const OUTPUT_OUT: &'static str = "out";

impl TPinCategory for AdapterDelayProcessData {
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
            // 蓄積する必要がある。
            INPUT_IN => Some(input::container_category::BUFFER_MONO_DYNAMIC),
            _ => None,
        }
    }
}

impl TSystemCategory for AdapterDelayProcessData {}
nz_define_time_tick_for!(AdapterDelayProcessData, true, true);

impl TProcess for AdapterDelayProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    /// 自分が処理可能なノードなのかを確認する。
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
        // 時間更新。またInputピンのリソース更新はしなくてもいい。
        self.common.elapsed_time = input.common.elapsed_time;
        self.common.process_input_pins_deprecated();

        match self.common.state {
            EProcessState::Stopped | EProcessState::Playing => self.update_state(input),
            _ => (),
        }
    }
}

impl TProcessItem for AdapterDelayProcessData {
    fn can_create_item(_setting: &ProcessItemCreateSetting) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_item(
        setting: &ProcessItemCreateSetting,
        system_setting: &InitializeSystemAccessor,
    ) -> anyhow::Result<TProcessItemPtr> {
        if let ENode::AdapterDelay(v) = setting.node {
            let item = Self {
                setting: setting.setting.clone(),
                common: ProcessControlItem::new(ProcessControlItemSetting {
                    specifier: ENodeSpecifier::AdapterDelay,
                    systems: &system_setting,
                }),
                internal: InternalInfo::new(v.clone()),
            };
            return Ok(SItemSPtr::new(item));
        }

        unreachable!("Unexpected branch");
    }
}

impl AdapterDelayProcessData {
    fn update_state(&mut self, input: &ProcessProcessorInput) {
        // まずRealtimeだけで。
        // @todo OFFLINE用はバッチにしたい。

        // Inputがあるかを確認する。なければ無視。
        let sample_rate = {
            let input_internal = self.common.get_input_internal(INPUT_IN).unwrap();
            let input = input_internal.buffer_mono_dynamic().unwrap();
            // もしインプットがきてなくて、Fsがセットされたなきゃなんもしない。
            if input.sample_rate == 0 {
                return;
            }

            input.sample_rate
        };

        self.internal.internal_time += input.common.frame_time;
        let time_offset = self.internal.internal_time - self.internal.last_process_time;
        let sample_counts = get_required_sample_count(time_offset, sample_rate);
        if sample_counts <= 0 {
            return;
        }

        // タイマーがまだ動作前なら何もしない。
        let old_internal_time = self.internal.last_process_time;
        self.internal.last_process_time = self.internal.internal_time;

        if self.internal.internal_time <= 0.0 {
            // ゼロ入りのバッファだけを作る。
            let buffer = vec![UniformedSample::MIN; sample_counts];
            self.common
                .insert_to_output_pin(
                    OUTPUT_OUT,
                    EProcessOutput::BufferMono(ProcessOutputBuffer::new(buffer, sample_rate)),
                )
                .unwrap();

            self.common.state = EProcessState::Playing;
            return;
        }

        // sample_countsからバッファ分をとる。
        // もしたりなきゃ作って返す。
        let pre_blank_counts = if old_internal_time < 0.0 {
            ((old_internal_time * -1.0) * (sample_rate as f64)).floor() as usize
        } else {
            0
        };
        debug_assert!(sample_counts >= pre_blank_counts);

        // 処理したものを渡す。
        let result = self.drain_buffer(input, sample_counts, pre_blank_counts);
        self.common
            .insert_to_output_pin(
                OUTPUT_OUT,
                EProcessOutput::BufferMono(ProcessOutputBuffer::new(result.buffer, sample_rate)),
            )
            .unwrap();

        // 状態確認
        if result.is_finished && input.is_children_all_finished() {
            self.common.state = EProcessState::Finished;
        } else {
            self.common.state = EProcessState::Playing;
        }
    }

    fn drain_buffer(
        &mut self,
        in_input: &ProcessProcessorInput,
        sample_counts: usize,
        pre_blank_counts: usize,
    ) -> DrainBufferResult {
        debug_assert!(sample_counts >= pre_blank_counts);
        let mut input_internal = self.common.get_input_internal_mut(INPUT_IN).unwrap();
        let input = input_internal.buffer_mono_dynamic_mut().unwrap();

        // `pre_blank_counts`が0より大きければバッファを作る。
        let mut buffer = vec![];
        if pre_blank_counts > 0 {
            buffer.resize(pre_blank_counts, UniformedSample::MIN);
        }
        let sample_counts = sample_counts - pre_blank_counts;

        // バッファ0補充分岐
        let now_buffer_len = input.buffer.len();
        let is_buffer_enough = now_buffer_len >= sample_counts;
        if !is_buffer_enough {
            buffer.append(&mut input.buffer.drain(..).collect_vec());
            buffer.resize(sample_counts, UniformedSample::MIN);
        }
        else {
            buffer.append(&mut input.buffer.drain(..sample_counts).collect_vec());
        }

        DrainBufferResult {
            buffer,
            is_finished: !is_buffer_enough && in_input.is_children_all_finished(),
        }
    }
}

#[derive(Default)]
struct DrainBufferResult {
    buffer: Vec<UniformedSample>,
    is_finished: bool,
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
