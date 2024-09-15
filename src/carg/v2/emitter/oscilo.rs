use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::{
    math::frequency::EFrequency,
    wave::{sample::UniformedSample, sine::emitter::SineUnitSampleEmitter},
};
use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::{EProcessOutput, EProcessState, EmitterRange, ProcessControlItem, ProcessOutputBuffer, ProcessProcessorInput, SItemSPtr, Setting, TProcess, TProcessItemPtr};
use crate::carg::v2::meta::node::ENode;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ESineWaveEmitterType {
    PinkNoise,
    WhiteNoise,
    Sine,
    Saw,
    Triangle,
    Square { duty_rate: f64 },
}

/// 正弦波を使って波形のバッファを作るための構造体
#[derive(Debug)]
pub struct SineWaveEmitterProcessData {
    setting: Setting,
    common: ProcessControlItem,
    emitter_type: ESineWaveEmitterType,
    /// `[0, 1]`まで
    intensity: f64,
    frequency: f64,
    range: EmitterRange,
    sample_elased_time: f64,
    /// 波形を出力するEmitter。
    emitter: Option<SineUnitSampleEmitter>,
}

impl SineWaveEmitterProcessData {
    pub fn create_from(node: &ENode, setting: &Setting) -> TProcessItemPtr {
        match node {
            ENode::EmitterPinkNoise { intensity, range } => {
                let item = SineWaveEmitterProcessData::new_pink(*intensity, *range, setting.clone());
                SItemSPtr::new(item)
            }
            ENode::EmitterWhiteNoise { intensity, range } => {
                let item = SineWaveEmitterProcessData::new_white(*intensity, *range, setting.clone());
                SItemSPtr::new(item)
            }
            ENode::EmitterSineWave {
                frequency,
                intensity,
                range,
            } => {
                let item = SineWaveEmitterProcessData::new_sine(*frequency, *intensity, *range, setting.clone());
                SItemSPtr::new(item)
            }
            ENode::EmitterSawtooth {
                frequency,
                intensity,
                range,
            } => {
                let item = SineWaveEmitterProcessData::new_saw(*frequency, *intensity, *range, setting.clone());
                SItemSPtr::new(item)
            }
            ENode::EmitterTriangle {
                frequency,
                intensity,
                range,
            } => {
                let item = SineWaveEmitterProcessData::new_triangle(*frequency, *intensity, *range, setting.clone());
                SItemSPtr::new(item)
            }
            ENode::EmitterSquare {
                frequency,
                duty_rate,
                intensity,
                range,
            } => {
                let item =
                    SineWaveEmitterProcessData::new_square(*frequency, *duty_rate, *intensity, *range, setting.clone());
                SItemSPtr::new(item)
            }
            _ => unreachable!("Unexpected branch."),
        }
    }
}

impl SineWaveEmitterProcessData {
    /// ピンクノイズの生成
    fn new_pink(intensity: f64, range: EmitterRange, setting: Setting) -> Self {
        Self {
            common: ProcessControlItem::new(ENodeSpecifier::EmitterPinkNoise),
            emitter_type: ESineWaveEmitterType::PinkNoise,
            intensity,
            frequency: 0.0,
            range,
            sample_elased_time: 0.0,
            setting,
            emitter: None,
        }
    }

    /// ホワイトノイズの生成
    fn new_white(intensity: f64, range: EmitterRange, setting: Setting) -> Self {
        Self {
            common: ProcessControlItem::new(ENodeSpecifier::EmitterWhiteNoise),
            emitter_type: ESineWaveEmitterType::WhiteNoise,
            intensity,
            frequency: 0.0,
            range,
            sample_elased_time: 0.0,
            setting,
            emitter: None,
        }
    }

    /// サイン波形の生成
    fn new_sine(frequency: EFrequency, intensity: f64, range: EmitterRange, setting: Setting) -> Self {
        Self {
            common: ProcessControlItem::new(ENodeSpecifier::EmitterSineWave),
            emitter_type: ESineWaveEmitterType::Sine,
            intensity,
            frequency: frequency.to_frequency(),
            range,
            sample_elased_time: 0.0,
            setting,
            emitter: None,
        }
    }

    /// ノコギリ波形の生成
    fn new_saw(frequency: EFrequency, intensity: f64, range: EmitterRange, setting: Setting) -> Self {
        Self {
            common: ProcessControlItem::new(ENodeSpecifier::EmitterSawtooth),
            emitter_type: ESineWaveEmitterType::Saw,
            intensity,
            frequency: frequency.to_frequency(),
            range,
            sample_elased_time: 0.0,
            setting,
            emitter: None,
        }
    }

    /// 三角波形の生成
    fn new_triangle(frequency: EFrequency, intensity: f64, range: EmitterRange, setting: Setting) -> Self {
        Self {
            common: ProcessControlItem::new(ENodeSpecifier::EmitterTriangle),
            emitter_type: ESineWaveEmitterType::Triangle,
            intensity,
            frequency: frequency.to_frequency(),
            range,
            sample_elased_time: 0.0,
            setting,
            emitter: None,
        }
    }

    /// 矩形波の生成
    fn new_square(
        frequency: EFrequency,
        duty_rate: f64,
        intensity: f64,
        range: EmitterRange,
        setting: Setting,
    ) -> Self {
        Self {
            common: ProcessControlItem::new(ENodeSpecifier::EmitterSquare),
            emitter_type: ESineWaveEmitterType::Square { duty_rate },
            intensity,
            frequency: frequency.to_frequency(),
            range,
            setting,
            emitter: None,
            sample_elased_time: 0.0,
        }
    }
}

impl TPinCategory for SineWaveEmitterProcessData {
    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの入力側のピンの名前を返す。
    fn get_input_pin_names() -> Vec<&'static str> { vec!["in"] }

    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの出力側のピンの名前を返す。
    fn get_output_pin_names() -> Vec<&'static str> { vec!["out"] }

    /// 関係ノードに書いているピンのカテゴリ（複数可）を返す。
    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            "in" => Some(pin_category::START),
            "out" => Some(pin_category::WAVE_BUFFER),
            _ => None,
        }
    }

    /// Inputピンのコンテナフラグ
    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            "in" => Some(input::container_category::EMPTY),
            _ => None,
        }
    }
}

impl SineWaveEmitterProcessData {
    /// 初期化する
    fn initialize(&mut self) {
        let emitter = match self.emitter_type {
            ESineWaveEmitterType::PinkNoise => SineUnitSampleEmitter::new_pinknoise(self.intensity),
            ESineWaveEmitterType::WhiteNoise => SineUnitSampleEmitter::new_whitenoise(self.intensity),
            ESineWaveEmitterType::Sine => {
                SineUnitSampleEmitter::new_sine(self.frequency, 0.0, self.intensity, self.setting.sample_rate as usize)
            }
            ESineWaveEmitterType::Saw => SineUnitSampleEmitter::new_sawtooth(
                self.frequency,
                0.0,
                self.intensity,
                self.setting.sample_rate as usize,
            ),
            ESineWaveEmitterType::Triangle => SineUnitSampleEmitter::new_triangle(
                self.frequency,
                0.0,
                self.intensity,
                self.setting.sample_rate as usize,
            ),
            ESineWaveEmitterType::Square { duty_rate } => SineUnitSampleEmitter::new_square(
                self.frequency,
                duty_rate,
                0.0,
                self.intensity,
                self.setting.sample_rate as usize,
            ),
        };
        self.emitter = Some(emitter);
    }

    /// 初期化した情報から設定分のOutputを更新する。
    fn next_samples(&mut self) -> Vec<UniformedSample> {
        assert!(self.emitter.is_some());

        // 設定のサンプル数ずつ吐き出す。
        // ただし今のと最終長さと比べて最終長さより長い分は0に埋める。
        let end_sample_index = {
            let ideal_add_time = self.setting.sample_count_frame as f64 / self.setting.sample_rate as f64;
            let ideal_next_time = self.common.elapsed_time + ideal_add_time;

            let mut add_time = ideal_add_time;
            if ideal_next_time > self.range.length {
                add_time = self.range.length - self.common.elapsed_time;
            }

            let samples = (add_time * self.setting.sample_rate as f64).ceil() as usize;
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

impl TProcess for SineWaveEmitterProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    /// 自分が処理可能なノードなのかを確認する。
    fn can_process(&self) -> bool {
        self.common.is_all_input_pins_update_notified()
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

        if self.common.state == EProcessState::Finished { return; }
        if self.common.state == EProcessState::Stopped {
            // 初期化する。
            self.initialize();
            assert!(self.emitter.is_some());
        }

        // 初期化した情報から設定分のOutputを更新する。
        // output_pinに入力。
        let buffer = self.next_samples();
        let elapsed_time = buffer.len() as f64 / self.setting.sample_rate as f64;
        self.common.insert_to_output_pin(
            "out",
            EProcessOutput::WaveBuffer(ProcessOutputBuffer {
                buffer,
                setting: self.setting.clone(),
                range: self.range,
            })
        ).unwrap();

        // 状態確認
        self.sample_elased_time += elapsed_time;
        if self.sample_elased_time < self.range.length {
            self.common.state = EProcessState::Playing;
        } else {
            self.common.state = EProcessState::Finished;
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
