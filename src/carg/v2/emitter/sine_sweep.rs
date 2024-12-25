use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::setting::Setting;
use crate::carg::v2::meta::system::TSystemCategory;
use crate::carg::v2::meta::tick::TTimeTickCategory;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::node::common::{EProcessState, ProcessControlItem};
use crate::carg::v2::{EProcessOutput, EmitterRange, ProcessItemCreateSetting, ProcessItemCreateSettingSystem, ProcessOutputBuffer, ProcessProcessorInput, SItemSPtr, TProcess, TProcessItem, TProcessItemPtr};
use crate::nz_define_time_tick_for;
use crate::wave::sine::emitter::SineUnitSampleEmitter;
use serde::{Deserialize, Serialize};
use soundprog::math::frequency::EFrequency;
use crate::carg::v2::meta::node::ENode;
use crate::wave::sample::UniformedSample;

///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaSineSweepInfo {
    from_frequency: EFrequency,
    to_frequency: EFrequency,
    range: EmitterRange,
    intensity: f64,
    sample_rate: usize,
}

#[derive(Debug)]
pub struct SineSweepEmitterProcessData {
    setting: Setting,
    common: ProcessControlItem,
    /// 設定情報
    info: MetaSineSweepInfo,
    sample_elapsed_time: f64,
    /// 波形を出力するEmitter。
    emitter: Option<SineUnitSampleEmitter>,
}

const INPUT_IN: &'static str = "in";
const OUTPUT_OUT: &'static str = "out";

impl TSystemCategory for SineSweepEmitterProcessData {}
nz_define_time_tick_for!(SineSweepEmitterProcessData, true, true);

impl TPinCategory for SineSweepEmitterProcessData {
    fn get_input_pin_names() -> Vec<&'static str> {
        vec![INPUT_IN]
    }

    fn get_output_pin_names() -> Vec<&'static str> {
        vec![OUTPUT_OUT]
    }

    /// 関係ノードに書いているピンのカテゴリ（複数可）を返す。
    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(pin_category::START),
            OUTPUT_OUT => Some(pin_category::BUFFER_MONO),
            _ => None,
        }
    }

    /// Inputピンのコンテナフラグ
    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(input::container_category::EMPTY),
            _ => None,
        }
    }
}

impl TProcessItem for SineSweepEmitterProcessData {
    fn can_create_item(_setting: &ProcessItemCreateSetting) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_item(
        setting: &ProcessItemCreateSetting,
        _system_setting: &ProcessItemCreateSettingSystem,
    ) -> anyhow::Result<TProcessItemPtr> {
        // これで関数実行は行うようにするけど変数は受け取らないことができる。
        let _is_ok = Self::can_create_item(&setting)?;

        if let ENode::EmitterSineSweep(v) = setting.node {
            let item = Self {
                setting: setting.setting.clone(),
                common: ProcessControlItem::new(ENodeSpecifier::EmitterSineSweep),
                info: v.clone(),
                sample_elapsed_time: 0.0,
                emitter: None,
            };

            return Ok(SItemSPtr::new(item));
        }

        unreachable!("Unexpected branch");
    }
}

impl TProcess for SineSweepEmitterProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    /// 自分が処理可能なノードなのかを確認する。
    fn can_process(&self) -> bool {
        true
    }

    /// 共用アイテムの参照を返す。
    fn get_common_ref(&self) -> &ProcessControlItem {
        &self.common
    }

    /// 共用アイテムの可変参照を返す。
    fn get_common_mut(&mut self) -> &mut ProcessControlItem {
        &mut self.common
    }

    fn try_process(&mut self, input: &ProcessProcessorInput) {
        // 時間更新。またInputピンのリソース更新はしなくてもいい。
        self.common.elapsed_time = input.common.elapsed_time;
        self.common.process_input_pins();

        if self.common.state == EProcessState::Finished {
            return;
        }
        if self.common.state == EProcessState::Stopped {
            // 初期化する。
            self.initialize();
            assert!(self.emitter.is_some());
        }

        // 初期化した情報から設定分のOutputを更新する。
        // output_pinに入力。
        let buffer = self.next_samples(input);
        let sample_rate = self.info.sample_rate;
        let elapsed_time = buffer.len() as f64 / sample_rate as f64;
        self.common
            .insert_to_output_pin(
                OUTPUT_OUT,
                EProcessOutput::BufferMono(ProcessOutputBuffer::new(buffer, sample_rate)),
            )
            .unwrap();

        // 状態確認
        self.sample_elapsed_time += elapsed_time;

        let range_length = self.info.range.length;
        if self.sample_elapsed_time < range_length {
            self.common.state = EProcessState::Playing;
        } else {
            self.common.state = EProcessState::Finished;
        }
    }
}

impl SineSweepEmitterProcessData {
    /// 初期化する
    fn initialize(&mut self) {
        let emitter = SineUnitSampleEmitter::new_sinesweep(
            self.info.from_frequency.to_frequency(),
            self.info.to_frequency.to_frequency(),
            self.info.range.length,
            self.info.intensity,
            self.info.sample_rate
        );
        self.emitter = Some(emitter);
    }

    /// 初期化した情報から設定分のOutputを更新する。
    fn next_samples(&mut self, _input: &ProcessProcessorInput) -> Vec<UniformedSample> {
        assert!(self.emitter.is_some());

        // 設定のサンプル数ずつ吐き出す。
        // ただし今のと最終長さと比べて最終長さより長い分は0に埋める。
        let end_sample_index = {
            let sample_rate = self.info.sample_rate;
            let ideal_add_time = (self.setting.sample_count_frame as f64) / (sample_rate as f64);
            let ideal_next_time = self.common.elapsed_time + ideal_add_time;

            let mut add_time = ideal_add_time;
            let range_length = self.info.range.length;
            if ideal_next_time > range_length {
                add_time = range_length - self.common.elapsed_time;
            }

            let samples = (add_time * sample_rate as f64).ceil() as usize;

            assert!(samples <= self.setting.sample_count_frame);
            samples
        };

        let mut samples = self.emitter.as_mut().unwrap().next_samples(self.setting.sample_count_frame);
        if end_sample_index < samples.len() {
            // [end_sample_index, len())までに0に埋める。
            samples
                .iter_mut()
                .skip(end_sample_index)
                .for_each(|v| *v = UniformedSample::MIN);
        }
        samples
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
