use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::system::{InitializeSystemAccessor, TSystemCategory};
use crate::carg::v2::meta::tick::TTimeTickCategory;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::node::common::{EProcessState, ProcessControlItemSetting};
use crate::carg::v2::{
    EProcessOutput, EmitterRange, ProcessControlItem, ProcessItemCreateSetting, ProcessOutputBuffer,
    ProcessProcessorInput, SItemSPtr, Setting, TProcess, TProcessItem, TProcessItemPtr,
};
use crate::nz_define_time_tick_for;
use crate::{
    math::frequency::EFrequency,
    wave::{sample::UniformedSample, sine::emitter::SineUnitSampleEmitter},
};
use serde::{Deserialize, Serialize};

/// ノイズなタイプの設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaSineNoiseInfo {
    /// `[0, 1]`まで
    intensity: f64,
    range: EmitterRange,
    sample_rate: usize,
}

/// ノイズではないタイプの設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaSineEmitterInfo {
    frequency: EFrequency,
    /// `[0, 1]`まで
    intensity: f64,
    range: EmitterRange,
    sample_rate: usize,
}

/// 矩形波の設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaSineSquareInfo {
    frequency: EFrequency,
    /// `[0, 1]`まで
    duty_rate: f64,
    /// `[0, 1]`まで
    intensity: f64,
    range: EmitterRange,
    sample_rate: usize,
}

#[derive(Debug, Clone)]
pub enum ESineWaveEmitterType {
    PinkNoise(MetaSineNoiseInfo),
    WhiteNoise(MetaSineNoiseInfo),
    Sine(MetaSineEmitterInfo),
    Saw(MetaSineEmitterInfo),
    Triangle(MetaSineEmitterInfo),
    Square(MetaSineSquareInfo),
}

impl ESineWaveEmitterType {
    pub fn range_length(&self) -> f64 {
        match self {
            ESineWaveEmitterType::PinkNoise(v) => v.range.length,
            ESineWaveEmitterType::WhiteNoise(v) => v.range.length,
            ESineWaveEmitterType::Sine(v) => v.range.length,
            ESineWaveEmitterType::Saw(v) => v.range.length,
            ESineWaveEmitterType::Triangle(v) => v.range.length,
            ESineWaveEmitterType::Square(v) => v.range.length,
        }
    }

    pub fn sample_rate(&self) -> usize {
        match self {
            ESineWaveEmitterType::PinkNoise(v) => v.sample_rate,
            ESineWaveEmitterType::WhiteNoise(v) => v.sample_rate,
            ESineWaveEmitterType::Sine(v) => v.sample_rate,
            ESineWaveEmitterType::Saw(v) => v.sample_rate,
            ESineWaveEmitterType::Triangle(v) => v.sample_rate,
            ESineWaveEmitterType::Square(v) => v.sample_rate,
        }
    }
}

/// 正弦波を使って波形のバッファを作るための構造体
#[derive(Debug)]
pub struct SineWaveEmitterProcessData {
    setting: Setting,
    common: ProcessControlItem,
    emitter_type: ESineWaveEmitterType,
    sample_elapsed_time: f64,
    /// 波形を出力するEmitter。
    emitter: Option<SineUnitSampleEmitter>,
}

const INPUT_IN: &'static str = "in";
const OUTPUT_OUT: &'static str = "out";

impl TProcessItem for SineWaveEmitterProcessData {
    fn can_create_item(_setting: &ProcessItemCreateSetting) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_item(
        setting: &ProcessItemCreateSetting,
        system_setting: &InitializeSystemAccessor,
    ) -> anyhow::Result<TProcessItemPtr> {
        Ok(match setting.node {
            ENode::EmitterPinkNoise(v) => {
                let item = SineWaveEmitterProcessData::new_pink(v, setting.setting.clone(), &system_setting);
                SItemSPtr::new(item)
            }
            ENode::EmitterWhiteNoise(v) => {
                let item = SineWaveEmitterProcessData::new_white(v, setting.setting.clone(), &system_setting);
                SItemSPtr::new(item)
            }
            ENode::EmitterSineWave(v) => {
                let item = SineWaveEmitterProcessData::new_sine(v, setting.setting.clone(), &system_setting);
                SItemSPtr::new(item)
            }
            ENode::EmitterSawtooth(v) => {
                let item = SineWaveEmitterProcessData::new_saw(v, setting.setting.clone(), &system_setting);
                SItemSPtr::new(item)
            }
            ENode::EmitterTriangle(v) => {
                let item = SineWaveEmitterProcessData::new_triangle(v, setting.setting.clone(), &system_setting);
                SItemSPtr::new(item)
            }
            ENode::EmitterSquare(v) => {
                let item = SineWaveEmitterProcessData::new_square(v, setting.setting.clone(), &system_setting);
                SItemSPtr::new(item)
            }
            _ => unreachable!("Unexpected branch."),
        })
    }
}

impl SineWaveEmitterProcessData {
    /// ピンクノイズの生成
    fn new_pink(info: &MetaSineNoiseInfo, setting: Setting, system_setting: &InitializeSystemAccessor) -> Self {
        Self {
            common: ProcessControlItem::new(ProcessControlItemSetting {
                specifier: ENodeSpecifier::EmitterPinkNoise,
                systems: &system_setting,
            }),
            emitter_type: ESineWaveEmitterType::PinkNoise(info.clone()),
            sample_elapsed_time: 0.0,
            setting,
            emitter: None,
        }
    }

    /// ホワイトノイズの生成
    fn new_white(info: &MetaSineNoiseInfo, setting: Setting, system_accessor: &InitializeSystemAccessor) -> Self {
        Self {
            common: ProcessControlItem::new(ProcessControlItemSetting {
                specifier: ENodeSpecifier::EmitterWhiteNoise,
                systems: &system_accessor,
            }),
            emitter_type: ESineWaveEmitterType::WhiteNoise(info.clone()),
            sample_elapsed_time: 0.0,
            setting,
            emitter: None,
        }
    }

    /// サイン波形の生成
    fn new_sine(info: &MetaSineEmitterInfo, setting: Setting, system_accessor: &InitializeSystemAccessor) -> Self {
        Self {
            common: ProcessControlItem::new(ProcessControlItemSetting {
                specifier: ENodeSpecifier::EmitterSineWave,
                systems: &system_accessor,
            }),
            emitter_type: ESineWaveEmitterType::Sine(info.clone()),
            sample_elapsed_time: 0.0,
            setting,
            emitter: None,
        }
    }

    /// ノコギリ波形の生成
    fn new_saw(info: &MetaSineEmitterInfo, setting: Setting, system_accessor: &InitializeSystemAccessor) -> Self {
        Self {
            common: ProcessControlItem::new(ProcessControlItemSetting {
                specifier: ENodeSpecifier::EmitterSawtooth,
                systems: &system_accessor,
            }),
            emitter_type: ESineWaveEmitterType::Saw(info.clone()),
            sample_elapsed_time: 0.0,
            setting,
            emitter: None,
        }
    }

    /// 三角波形の生成
    fn new_triangle(info: &MetaSineEmitterInfo, setting: Setting, system_accessor: &InitializeSystemAccessor) -> Self {
        Self {
            common: ProcessControlItem::new(ProcessControlItemSetting {
                specifier: ENodeSpecifier::EmitterTriangle,
                systems: &system_accessor,
            }),
            emitter_type: ESineWaveEmitterType::Triangle(info.clone()),
            sample_elapsed_time: 0.0,
            setting,
            emitter: None,
        }
    }

    /// 矩形波の生成
    fn new_square(info: &MetaSineSquareInfo, setting: Setting, system_accessor: &InitializeSystemAccessor) -> Self {
        Self {
            common: ProcessControlItem::new(ProcessControlItemSetting {
                specifier: ENodeSpecifier::EmitterSquare,
                systems: &system_accessor,
            }),
            emitter_type: ESineWaveEmitterType::Square(info.clone()),
            setting,
            emitter: None,
            sample_elapsed_time: 0.0,
        }
    }
}

impl TPinCategory for SineWaveEmitterProcessData {
    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの入力側のピンの名前を返す。
    fn get_input_pin_names() -> Vec<&'static str> {
        vec![INPUT_IN]
    }

    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの出力側のピンの名前を返す。
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

impl SineWaveEmitterProcessData {
    /// 初期化する
    fn initialize(&mut self) {
        let emitter = match &self.emitter_type {
            ESineWaveEmitterType::PinkNoise(v) => SineUnitSampleEmitter::new_pinknoise(v.intensity),
            ESineWaveEmitterType::WhiteNoise(v) => SineUnitSampleEmitter::new_whitenoise(v.intensity),
            ESineWaveEmitterType::Sine(v) => {
                SineUnitSampleEmitter::new_sine(v.frequency.to_frequency(), 0.0, v.intensity, v.sample_rate)
            }
            ESineWaveEmitterType::Saw(v) => {
                SineUnitSampleEmitter::new_sawtooth(v.frequency.to_frequency(), 0.0, v.intensity, v.sample_rate)
            }
            ESineWaveEmitterType::Triangle(v) => {
                SineUnitSampleEmitter::new_triangle(v.frequency.to_frequency(), 0.0, v.intensity, v.sample_rate)
            }
            ESineWaveEmitterType::Square(v) => SineUnitSampleEmitter::new_square(
                v.frequency.to_frequency(),
                v.duty_rate,
                0.0,
                v.intensity,
                v.sample_rate,
            ),
        };
        self.emitter = Some(emitter);
    }

    /// 初期化した情報から設定分のOutputを更新する。
    fn next_samples(&mut self, input: &ProcessProcessorInput) -> Vec<UniformedSample> {
        assert!(self.emitter.is_some());

        // 設定のサンプル数ずつ吐き出す。
        // ただし今のと最終長さと比べて最終長さより長い分は0に埋める。
        let required_sample_count = input.get_realtime_required_samples(self.emitter_type.sample_rate());
        if required_sample_count < 0 {
            return vec![];
        }

        let end_sample_index = {
            let ideal_add_time = (required_sample_count as f64) / (self.emitter_type.sample_rate() as f64);
            let ideal_next_time = self.common.elapsed_time + ideal_add_time;

            let mut add_time = ideal_add_time;
            let range_length = self.emitter_type.range_length();
            if ideal_next_time > range_length {
                add_time = range_length - self.common.elapsed_time;
            }

            let sample_rate = self.emitter_type.sample_rate();
            let samples = (add_time * sample_rate as f64).ceil() as usize;

            debug_assert!(samples <= required_sample_count);
            samples
        };

        let mut samples = self.emitter.as_mut().unwrap().next_samples(required_sample_count);
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

impl TSystemCategory for SineWaveEmitterProcessData {}
nz_define_time_tick_for!(SineWaveEmitterProcessData, true, true);

impl TProcess for SineWaveEmitterProcessData {
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
        if buffer.is_empty() {
            return;
        }

        let sample_rate = self.emitter_type.sample_rate();
        let elapsed_time = buffer.len() as f64 / sample_rate as f64;
        self.common
            .insert_to_output_pin(
                OUTPUT_OUT,
                EProcessOutput::BufferMono(ProcessOutputBuffer::new(buffer, sample_rate)),
            )
            .unwrap();

        // 状態確認
        self.sample_elapsed_time += elapsed_time;
        let range_length = self.emitter_type.range_length();
        if self.sample_elapsed_time < range_length {
            self.common.state = EProcessState::Playing;
        } else {
            self.common.state = EProcessState::Finished;
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
