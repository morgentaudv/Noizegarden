use crate::{
    math::frequency::EFrequency,
    wave::{
        sample::UniformedSample,
        sine::setting::{EFrequencyItem, WaveFormatSetting, WaveSound, WaveSoundSettingBuilder},
    },
};

use super::{
    EProcessResult, EProcessState, ESineWaveEmitterType, EmitterRange, ProcessControlItem, ProcessInput,
    ProcessOutputBuffer, Setting, TInputNoneOutputBuffer, TProcess,
};

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
    /// 処理後に出力情報が保存されるところ。
    output: Option<ProcessOutputBuffer>,
}

impl SineWaveEmitterProcessData {
    /// ピンクノイズの生成
    pub fn new_pink(intensity: f64, range: EmitterRange, setting: Setting) -> Self {
        Self {
            common: ProcessControlItem::new(),
            emitter_type: ESineWaveEmitterType::PinkNoise,
            intensity: intensity,
            frequency: 0.0,
            range: range,
            setting: setting,
            output: None,
        }
    }

    /// ホワイトノイズの生成
    pub fn new_white(intensity: f64, range: EmitterRange, setting: Setting) -> Self {
        Self {
            common: ProcessControlItem::new(),
            emitter_type: ESineWaveEmitterType::WhiteNoise,
            intensity: intensity,
            frequency: 0.0,
            range: range,
            setting: setting,
            output: None,
        }
    }

    /// サイン波形の生成
    pub fn new_sine(frequency: EFrequency, intensity: f64, range: EmitterRange, setting: Setting) -> Self {
        Self {
            common: ProcessControlItem::new(),
            emitter_type: ESineWaveEmitterType::Sine,
            intensity: intensity,
            frequency: frequency.to_frequency(),
            range: range,
            setting: setting,
            output: None,
        }
    }

    /// ノコギリ波形の生成
    pub fn new_saw(frequency: EFrequency, intensity: f64, range: EmitterRange, setting: Setting) -> Self {
        Self {
            common: ProcessControlItem::new(),
            emitter_type: ESineWaveEmitterType::Saw,
            intensity: intensity,
            frequency: frequency.to_frequency(),
            range: range,
            setting: setting,
            output: None,
        }
    }

    /// 三角波形の生成
    pub fn new_triangle(frequency: EFrequency, intensity: f64, range: EmitterRange, setting: Setting) -> Self {
        Self {
            common: ProcessControlItem::new(),
            emitter_type: ESineWaveEmitterType::Triangle,
            intensity: intensity,
            frequency: frequency.to_frequency(),
            range: range,
            setting: setting,
            output: None,
        }
    }

    /// 矩形波の生成
    pub fn new_square(
        frequency: EFrequency,
        duty_rate: f64,
        intensity: f64,
        range: EmitterRange,
        setting: Setting,
    ) -> Self {
        Self {
            common: ProcessControlItem::new(),
            emitter_type: ESineWaveEmitterType::Square { duty_rate },
            intensity: intensity,
            frequency: frequency.to_frequency(),
            range: range,
            setting: setting,
            output: None,
        }
    }
}

impl SineWaveEmitterProcessData {
    pub fn initialize(&mut self) {
        let frequency = match self.emitter_type {
            ESineWaveEmitterType::PinkNoise => EFrequencyItem::PinkNoise,
            ESineWaveEmitterType::WhiteNoise => EFrequencyItem::WhiteNoise,
            ESineWaveEmitterType::Sine => EFrequencyItem::Constant {
                frequency: self.frequency,
            },
            ESineWaveEmitterType::Saw => EFrequencyItem::Constant {
                frequency: self.frequency,
            },
            ESineWaveEmitterType::Triangle => EFrequencyItem::Triangle {
                frequency: self.frequency,
            },
            ESineWaveEmitterType::Square { duty_rate } => EFrequencyItem::Square {
                frequency: self.frequency,
                duty_rate,
            },
        };
    }
}

impl TInputNoneOutputBuffer for SineWaveEmitterProcessData {
    /// 自分のタイムスタンプを返す。
    fn get_timestamp(&self) -> i64 {
        self.common.process_timestamp
    }

    fn get_output(&self) -> ProcessOutputBuffer {
        assert!(self.output.is_some());
        self.output.as_ref().unwrap().clone()
    }

    fn set_child_count(&mut self, count: usize) {
        self.common.child_count = count;
    }
}

impl TProcess for SineWaveEmitterProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    fn try_process(&mut self, input: &ProcessInput) -> EProcessResult {
        //match self.common.state {
        //    EProcessState::Stopped => {
        //        // 初期化する。
        //        self.initialize();

        //        // 初期化した情報から設定分のOutputを更新する。
        //
        //    },
        //    EProcessState::Playing => todo!(),
        //    EProcessState::Finished => EProcessState::Finished,
        //}

        let frequency = match self.emitter_type {
            ESineWaveEmitterType::PinkNoise => EFrequencyItem::PinkNoise,
            ESineWaveEmitterType::WhiteNoise => EFrequencyItem::WhiteNoise,
            ESineWaveEmitterType::Sine => EFrequencyItem::Constant {
                frequency: self.frequency,
            },
            ESineWaveEmitterType::Saw => EFrequencyItem::Constant {
                frequency: self.frequency,
            },
            ESineWaveEmitterType::Triangle => EFrequencyItem::Triangle {
                frequency: self.frequency,
            },
            ESineWaveEmitterType::Square { duty_rate } => EFrequencyItem::Square {
                frequency: self.frequency,
                duty_rate,
            },
        };
        let sound_setting = WaveSoundSettingBuilder::default()
            .frequency(frequency)
            .length_sec(self.range.length as f32)
            .intensity(self.intensity)
            .build()
            .unwrap();

        let format = WaveFormatSetting {
            samples_per_sec: self.setting.sample_rate as u32,
            bits_per_sample: crate::wave::sine::setting::EBitsPerSample::Bits16,
        };
        let sound = WaveSound::from_setting(&format, &sound_setting);

        let mut buffer: Vec<UniformedSample> = vec![];
        for fragment in sound.sound_fragments {
            buffer.extend(&fragment.buffer);
        }

        // outputのどこかに保持する。
        self.output = Some(ProcessOutputBuffer {
            buffer,
            setting: self.setting.clone(),
            range: self.range,
        });

        // 状態変更。
        self.common.state = EProcessState::Finished;
        self.common.process_timestamp += 1;
        return EProcessResult::Finished;
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
