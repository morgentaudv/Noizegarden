use crate::carg::v2::meta::input::{BufferMonoDynamicItem, BufferStereoDynamicItem, EInputContainerCategoryFlag};
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::output::EProcessOutputContainer;
use crate::carg::v2::meta::process::{process_category, EProcessCategoryFlag, TProcessCategory};
use crate::carg::v2::meta::system::{system_category, ESystemCategoryFlag, TSystemCategory};
use crate::carg::v2::meta::tick::TTimeTickCategory;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::node::common::EProcessState;
use crate::carg::v2::{
    ProcessControlItem, ProcessItemCreateSetting, ProcessItemCreateSettingSystem, ProcessProcessorInput, SItemSPtr,
    TProcess, TProcessItem, TProcessItemPtr,
};
use crate::device::{AudioDeviceProxyWeakPtr, EDrainedChannelBuffers};
use crate::nz_define_time_tick_for;
use crate::wave::sample::UniformedSample;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::cell::UnsafeCell;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaOutputDeviceInfo {}

#[derive(Debug)]
struct InternalData {}

#[derive(Debug)]
pub struct OutputDeviceProcessData {
    /// 共通アイテム
    common: ProcessControlItem,
    /// デバイスに接近するためのプロキシ
    device_proxy: AudioDeviceProxyWeakPtr,
    /// 内部用データ
    #[allow(dead_code)]
    internal: InternalData,
}

const INPUT_IN: &'static str = "in";

impl TPinCategory for OutputDeviceProcessData {
    fn get_input_pin_names() -> Vec<&'static str> {
        vec![INPUT_IN]
    }

    fn get_output_pin_names() -> Vec<&'static str> {
        vec![]
    }

    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(pin_category::BUFFER_MONO),
            _ => None,
        }
    }

    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(input::container_category::OUTPUT_DEVICE),
            _ => None,
        }
    }
}

impl TSystemCategory for OutputDeviceProcessData {
    fn get_dependent_system_categories() -> ESystemCategoryFlag {
        system_category::AUDIO_DEVICE
    }
}
nz_define_time_tick_for!(OutputDeviceProcessData, false, true);

impl TProcessCategory for OutputDeviceProcessData {
    fn get_process_category() -> EProcessCategoryFlag {
        process_category::BUS_MASTER_OUTPUT
    }
}

impl TProcess for OutputDeviceProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn can_process(&self) -> bool {
        self.common.is_all_input_pins_update_notified()
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

impl OutputDeviceProcessData {
    fn update_state(&mut self, input: &ProcessProcessorInput) {
        // ほかのoutputノードと違って、常に何かのバッファを流す必要がある。
        // もし新規の音源バッファがなかったら、0にして入れる必要がある。
        //
        // 持っているプロキシに最終処理したバッファを送る。
        // その前に今どのぐらいのバッファを必要とするかを返してから、その量に応じて分量をとる。

        // もしデバイスが死んだら処理してはいけないし、処理中断する。
        //
        // ただしサンプルフォーマットはここで変更しない。
        // 送った先でなんとかやってくれる。
        // いったん[`UniformedSample`]自体はf32だとみなす。
        let device = self.device_proxy.upgrade();
        if device.is_none() {
            self.common.state = EProcessState::Finished;
            return;
        }

        // 各チャンネルのバッファ区間に送信するためのサンプルの数を取得する。
        // 24-12-15 この辺はminiaudioライブラリの実装にゆだねられる。
        // 24-12-17
        {
            let item = self.common.get_input_internal_mut(INPUT_IN).unwrap();
            if item.output_dynamic().is_none() {
                return;
            }
        }

        // ただしサンプルフォーマットはここで変更しない。
        // 送った先でなんとかやってくれる。
        // いったん[`UniformedSample`]自体はf32だとみなす。
        let mut is_all_zero = false;
        let this_pointer = UnsafeCell::new(self as *mut Self);
        let all_zero_pointer = UnsafeCell::new(&mut is_all_zero as *mut bool);

        // 与えた関数は必ずこの関数の中で実行するロジックになっている。
        // のでthis(self)も無事なはずなので、ちょっと方法がわるいけどこうして中に渡す。
        let send_buffer_fn = move |frame_count| {
            // 1. inputをforにして、frame_iを増加する。
            // 2. frame_iがframe_countより同じか大きければ、抜ける。
            let this = unsafe { &mut **this_pointer.get() };

            // 送信用のバッファとすべてゼロかを取得。
            let (channel_buffers, is_all_zero) = {
                let mut item = this.common.get_input_internal_mut(INPUT_IN).unwrap();
                let item = item.output_dynamic_mut().unwrap();

                match item {
                    EOutputDeviceInput::Mono(v) => {
                        let (channel, is_all_zero) = get_drained_buffer_from(&mut v.buffer, frame_count);

                        (EDrainedChannelBuffers::Mono { channel }, is_all_zero)
                    }
                    EOutputDeviceInput::Stereo(v) => {
                        let (ch_left, left_all_zero) = get_drained_buffer_from(&mut v.ch_left, frame_count);
                        let (ch_right, right_all_zero) = get_drained_buffer_from(&mut v.ch_right, frame_count);

                        (
                            EDrainedChannelBuffers::Stereo { ch_left, ch_right },
                            left_all_zero & right_all_zero,
                        )
                    }
                }
            };

            // 更新。
            let all_zero = unsafe { &mut **all_zero_pointer.get() };
            *all_zero = is_all_zero;

            channel_buffers
        };

        // critical section
        {
            let device = device.as_ref().unwrap();
            let proxy = device.lock().unwrap();
            proxy.send_sample_buffer_with(send_buffer_fn);
        }

        // 24-12-11
        // もしレンダリングがすべて終わって、要求サンプルがすべて0うめなら終了する。
        // 自分を終わるかしないかのチェック
        if input.is_children_all_finished() && is_all_zero {
            self.common.state = EProcessState::Finished;
            return;
        } else {
            self.common.state = EProcessState::Playing;
            return;
        }
    }
}

impl TProcessItem for OutputDeviceProcessData {
    fn can_create_item(_setting: &ProcessItemCreateSetting) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_item(
        setting: &ProcessItemCreateSetting,
        system_setting: &ProcessItemCreateSettingSystem,
    ) -> anyhow::Result<TProcessItemPtr> {
        match setting.node {
            ENode::OutputDevice(_info) => {
                let item = Self {
                    common: ProcessControlItem::new(ENodeSpecifier::OutputDevice),
                    device_proxy: system_setting.audio_device.unwrap().clone(),
                    internal: InternalData {},
                };
                Ok(SItemSPtr::new(item))
            }
            _ => unreachable!("Unexpected branch."),
        }
    }
}

// ----------------------------------------------------------------------------
// Helper Functions
// ----------------------------------------------------------------------------

///
fn get_drained_buffer_from(buffer: &mut Vec<UniformedSample>, required_samples: usize) -> (Vec<UniformedSample>, bool) {
    let drain_length = required_samples.min(buffer.len());
    let zero_length = if drain_length < required_samples {
        required_samples - drain_length
    } else {
        0
    };

    // バッファにする。
    let mut result_buffer = buffer.drain(..drain_length).collect_vec();
    if zero_length > 0 {
        result_buffer.resize(required_samples, UniformedSample::MIN);
    }

    (result_buffer, zero_length == required_samples)
}

// ----------------------------------------------------------------------------
// EOutputDeviceInput
// ----------------------------------------------------------------------------

/// [`OutputDeviceProcessData`]の入力用コンテナの中身
#[derive(Debug, Clone)]
pub enum EOutputDeviceInput {
    Mono(BufferMonoDynamicItem),
    Stereo(BufferStereoDynamicItem),
}

impl EOutputDeviceInput {
    /// 今のセッティングで`output`が受け取れるか？
    pub fn can_support(&self, output: &EProcessOutputContainer) -> bool {
        match self {
            Self::Mono(_) => match output {
                EProcessOutputContainer::BufferMono(_) => true,
                _ => false,
            },
            Self::Stereo(_) => match output {
                EProcessOutputContainer::BufferStereo(_) => true,
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
                *self = Self::Mono(BufferMonoDynamicItem::new());
            }
            EProcessOutputContainer::BufferStereo(_) => {
                *self = Self::Stereo(BufferStereoDynamicItem::new());
            }
            _ => unreachable!("Unexpected branch"),
        }
    }

    /// 種類をかえずに中身だけをリセットする。
    pub fn reset(&mut self) {
        match self {
            Self::Mono(v) => {
                v.buffer.clear();
            }
            Self::Stereo(v) => {
                v.ch_left.clear();
                v.ch_right.clear();
            }
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
