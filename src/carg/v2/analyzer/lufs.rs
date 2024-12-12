use crate::carg::v2::filter::iir_compute_sample;
use crate::carg::v2::meta::input::{EInputContainerCategoryFlag, EProcessInputContainer};
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::setting::Setting;
use crate::carg::v2::meta::system::TSystemCategory;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::{
    EProcessOutput, EProcessState, ProcessControlItem, ProcessItemCreateSetting, ProcessItemCreateSettingSystem,
    ProcessOutputText, ProcessProcessorInput, SItemSPtr, TProcess, TProcessItem, TProcessItemPtr,
};
use crate::wave::sample::UniformedSample;
use serde::{Deserialize, Serialize};

mod hz48000 {
    /// HRTFのIIRフィルター（おおむねハイシェルブ）
    pub const IIR_AS: [f64; 3] = [1.0, -1.69065929318241, 0.73248077421585];
    pub const IIR_BS: [f64; 3] = [1.53512485958697, -2.69169618940638, 1.19839281065285];

    /// RLBのIIRフィルター（ハイパス）
    pub const IIR_CS: [f64; 3] = [1.0, -1.99004745483398, 0.99007225036621];
    pub const IIR_DS: [f64; 3] = [1.0, -2.0, 1.0];
}

/// 44.1kHzの時の数値の参考は
/// https://www.wizard-notes.com/entry/music-analysis/k-weighting
mod hz44100 {
    pub const IIR_AS: [f64; 3] = [1.0, -1.69066543, 0.73246971];
    pub const IIR_BS: [f64; 3] = [1.53517731, -2.69174966, 1.19837662];

    pub const IIR_CS: [f64; 3] = [1.0, -1.99431068, 0.99433471];
    pub const IIR_DS: [f64; 3] = [0.99716135, -1.99432269, 0.99716135];
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
const OUTPUT_INFO: &'static str = "out_info";
const OUTPUT_LUFS: &'static str = "out_lufs";

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
        vec![OUTPUT_INFO, OUTPUT_LUFS]
    }

    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(pin_category::BUFFER_MONO),
            OUTPUT_INFO => Some(pin_category::TEXT),
            OUTPUT_LUFS => Some(pin_category::TEXT),
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

impl TSystemCategory for AnalyzeLUFSProcessData {}

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

impl TProcessItem for AnalyzeLUFSProcessData {
    fn can_create_item(setting: &ProcessItemCreateSetting) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_item(
        setting: &ProcessItemCreateSetting,
        system_setting: &ProcessItemCreateSettingSystem,
    ) -> anyhow::Result<TProcessItemPtr> {
        // これで関数実行は行うようにするけど変数は受け取らないことができる。
        let _is_ok = Self::can_create_item(&setting)?;

        if let ENode::AnalyzerLUFS(v) = setting.node {
            let item = Self {
                setting: setting.setting.clone(),
                common: ProcessControlItem::new(ENodeSpecifier::AnalyzerLUFS),
                info: v.clone(),
                internal: InternalInfo::default(),
            };

            return Ok(SItemSPtr::new(item));
        }

        unreachable!("Unexpected branch");
    }
}

impl AnalyzeLUFSProcessData {
    fn update_state(&mut self, in_input: &ProcessProcessorInput) {
        let can_process = self.update_input_buffer();
        if !can_process {
            // 自分を終わるかしないかのチェック
            if in_input.is_children_all_finished() {
                self.common.state = EProcessState::Finished;
            }

            return;
        }

        let sample_rate = if self.info.use_input {
            self.setting.sample_rate as f64
        } else {
            self.setting.sample_rate as f64
        };
        let (iir_as, iir_bs, iir_cs, iir_ds) = {
            match sample_rate {
                48000.0 => (hz48000::IIR_AS, hz48000::IIR_BS, hz48000::IIR_CS, hz48000::IIR_DS),
                44100.0 => (hz44100::IIR_AS, hz44100::IIR_BS, hz44100::IIR_CS, hz44100::IIR_DS),
                _ => unreachable!("Unexpected branch"),
            }
        };

        // `block_length`から処理ができるかを確認する。
        // もしインプットが終わったなら、尺が足りなくてもそのまま処理する。
        let block_sample_len = (self.info.block_length as f64 * sample_rate).ceil() as usize;

        // k-weightとRLBを通す。
        let after_k_weight = {
            let start_i = self.internal.next_start_i;
            let item = self.common.get_input_internal(INPUT_IN).unwrap();
            let item = item.buffer_mono_dynamic().unwrap();
            let buffer = &item.buffer;
            let sample_range = start_i..(block_sample_len + start_i);

            let mut output_buffer = vec![];
            output_buffer.resize(sample_range.len(), UniformedSample::default());

            for sample_i in sample_range {
                let output_i = sample_i - start_i;
                iir_compute_sample(output_i, sample_i, &mut output_buffer, buffer, &iir_as, &iir_bs);
            }

            output_buffer
        };
        let after_rlb = {
            let mut output_buffer = vec![];
            output_buffer.resize(block_sample_len, UniformedSample::default());

            for sample_i in 0..block_sample_len {
                let output_i = sample_i;
                iir_compute_sample(output_i, sample_i, &mut output_buffer, &after_k_weight, &iir_cs, &iir_ds);
            }

            output_buffer
        };

        // そしてRMS平均する。
        let block_lufs = {
            let after_rms = {
                let sum = after_rlb.into_iter().map(|v| v.to_f64().powi(2)).sum::<f64>();
                sum / block_sample_len as f64
            };
            if after_rms.is_subnormal() {
                -1280.0
            } else {
                (after_rms.log10() * 10.0) - 0.691
            }
        };

        // 処理が終わったあとの更新。
        let slide_sample_len = (self.info.slide_length as f64 * sample_rate).floor() as usize;
        self.internal.next_start_i += slide_sample_len;

        if self.internal.processed_time <= 0.0 {
            let sample_range_time = (block_sample_len as f64) / sample_rate;
            self.internal.processed_time += sample_range_time;
        } else {
            let proceed_time = (slide_sample_len as f64) / sample_rate;
            self.internal.processed_time += proceed_time;
        }

        // out_info関連出力処理
        if self.common.is_output_pin_connected(OUTPUT_INFO) {
            let mut log = "".to_owned();
            log += &format!("{:?}, {block_lufs}-dB", &self.internal);

            self.common
                .insert_to_output_pin(OUTPUT_INFO, EProcessOutput::Text(ProcessOutputText { text: log }))
                .unwrap();
        }

        // out_freq関連出力処理
        if self.common.is_output_pin_connected(OUTPUT_LUFS) {
            //dbg!(&self.internal, block_lufs);
        }

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
        // `block_length`から処理ができるかを確認する。
        // もしインプットが終わったなら、尺が足りなくてもそのまま処理する。
        let sample_rate = if self.info.use_input {
            self.setting.sample_rate as f64
        } else {
            self.setting.sample_rate as f64
        };
        let block_sample_len = (self.info.block_length as f64 * sample_rate).ceil() as usize;

        // 処理するためのバッファが十分じゃないと処理できない。
        let is_buffer_enough = match &*self.common.get_input_internal(INPUT_IN).unwrap() {
            EProcessInputContainer::BufferMonoDynamic(v) => {
                v.buffer.len() > (block_sample_len + self.internal.next_start_i)
            }
            _ => false,
        };
        if !is_buffer_enough {
            return false;
        }

        // もしinternalのindexが0じゃなきゃ前の分を縮められる。
        let mut item = self.common.get_input_internal_mut(INPUT_IN).unwrap();
        let item = &mut item.buffer_mono_dynamic_mut().unwrap();
        if self.internal.next_start_i != 0 {
            // 前を削除する。
            item.buffer.drain(..self.internal.next_start_i);
            self.internal.next_start_i = 0;
        }

        true
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
