use derive_builder::Builder;
use itertools::Itertools;
use std::f64::consts::PI;

use super::{sample::UniformedSample, PI2};

/// 各サンプルの量子化レベルを表す。
///
/// サウンドのサンプルの量子化については、
/// [Quantization_(signal_processing)](https://en.wikipedia.org/wiki/Quantization_(signal_processing)) ページを参照すること。
#[derive(Debug, Clone, Copy)]
pub enum EBitsPerSample {
    /// 65,536レベル。
    ///
    /// Waveの場合、エクスポートした音源はサンプルデータは`[-32,768, 32,767)`の範囲を持つ。
    Bits16,
}

impl EBitsPerSample {
    /// [`EBitsPerSample`]を数字[`u32`]に変換する。
    ///
    /// ```
    /// # use soundprog::wave::setting::EBitsPerSample;
    /// let bps = EBitsPerSample::Bits16;
    /// let digits = bps.to_u32();
    /// assert_eq!(digits, 16u32);
    /// ```
    pub const fn to_u32(self) -> u32 {
        match self {
            EBitsPerSample::Bits16 => 16u32,
        }
    }

    /// [`EBitsPerSample`]をバイトサイズに合わせて変換する。
    ///
    /// ```
    /// # use soundprog::wave::setting::EBitsPerSample;
    /// let bps = EBitsPerSample::Bits16;
    /// let bytes = bps.to_byte_size();
    /// assert_eq!(bytes, 2);
    /// ```
    pub const fn to_byte_size(self) -> usize {
        (self.to_u32() as usize) / 8
    }
}

///
#[derive(Debug, Clone, Copy)]
pub struct WaveFormatSetting {
    pub samples_per_sec: u32,
    pub bits_per_sample: EBitsPerSample,
}

#[derive(Debug, Clone, Copy)]
pub enum EIntensityControlItem {
    ConstantMultifly(f64),
    Fade {
        start_time: f64,
        length: f64,
        start_factor: f64,
        end_factor: f64,
    },
    Exp {
        start_time: f64,
        length: Option<f64>,
        coefficient: f64,
    },
}

impl EIntensityControlItem {
    const DEFAULT_FACTOR: f64 = 1.0;

    ///
    pub fn calculate_factor(&self, relative_time: f64, sound_setting: &WaveSoundSetting) -> f64 {
        match self {
            EIntensityControlItem::ConstantMultifly(v) => *v,
            EIntensityControlItem::Fade {
                start_time,
                length,
                start_factor,
                end_factor,
            } => {
                let end_time = start_time + length;
                if relative_time < *start_time || relative_time > end_time {
                    return Self::DEFAULT_FACTOR;
                }
                if *length <= 0.0 {
                    return Self::DEFAULT_FACTOR;
                }

                let lerp_f = ((relative_time - start_time) / length).clamp(0.0, 1.0);
                (end_factor - start_factor) * lerp_f + start_factor
            }
            EIntensityControlItem::Exp {
                start_time,
                length,
                coefficient,
            } => {
                // もしlengthに値があれば、end_timeがあるとみなす。
                match length {
                    Some(length) => {
                        let end_time = start_time + length;
                        if relative_time < *start_time || relative_time > end_time {
                            return Self::DEFAULT_FACTOR;
                        }
                        if *length <= 0.0 {
                            return Self::DEFAULT_FACTOR;
                        }
                    }
                    None => {
                        if relative_time < *start_time {
                            return Self::DEFAULT_FACTOR;
                        }
                    }
                }

                (coefficient * relative_time).exp()
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EFrequencyItem {
    Constant { frequency: f64 },
    Chirp { start_frequency: f64, end_frequency: f64 },
    Sawtooth { frequency: f64 },
}

impl Default for EFrequencyItem {
    fn default() -> Self {
        Self::Constant { frequency: 0.0 }
    }
}

///
#[derive(Default, Debug, Clone, Builder)]
#[builder(default)]
pub struct WaveSoundSetting {
    pub frequency: EFrequencyItem,
    pub phase: f32,
    pub length_sec: f32,
    pub intensity: f64,
    pub oscillator_vibrato: Option<OscillatorVibrato>,
    pub intensity_control_items: Vec<EIntensityControlItem>,
}

///
#[derive(Debug, Clone, Copy)]
struct CalculatedSamplesCount {
    begin_index: usize,
    length: usize,
}

impl CalculatedSamplesCount {
    pub fn end_index(&self) -> usize {
        self.begin_index + self.length
    }
}

/// サイン波形を含んだ基本音波情報
#[derive(Debug, Clone)]
pub struct SoundFragment {
    pub sound: WaveSoundSetting,
    samples_count: CalculatedSamplesCount,
    pub buffer: Vec<UniformedSample>,
}

impl SoundFragment {
    /// サンプル/秒と長さからサンプルの数を計算する。
    fn calc_samples_count(samples_per_sec: u32, sound_setting: &WaveSoundSetting) -> CalculatedSamplesCount {
        // サンプル/秒と長さからサンプルの数を計算する。
        // 今はMONOなので1:1でマッチできる。
        //
        // 小数点がある場合には、総サンプルは足りないよりは余ったほうが都合が良いかもしれない。
        let samples = samples_per_sec as f32;
        let begin_index = 0usize;
        let length = (samples * sound_setting.length_sec).ceil() as usize;

        CalculatedSamplesCount { begin_index, length }
    }

    /// 設定数値から単一サウンドを生成して返す。
    pub fn from_setting(format: &WaveFormatSetting, sound: &WaveSoundSetting) -> Option<Self> {
        // サンプル/秒と長さからサンプルの数を計算する。
        // 今はMONOなので1:1でマッチできる。
        // 小数点がある場合には、総サンプルは足りないよりは余ったほうが都合が良いかもしれない。
        let samples_count = Self::calc_samples_count(format.samples_per_sec, sound);

        // Sin波形に入れる値はf64として計算する。
        // そしてu32に変換する。最大値は`[2^32 - 1]`である。
        let coefficient = PI2 / (format.samples_per_sec as f64);
        let samples = {
            let mut samples = vec![];
            samples.reserve(samples_count.length);

            match &sound.frequency {
                EFrequencyItem::Constant { frequency } => {
                    for unittime in 0..samples_count.length {
                        // 振幅と周波数のエンベロープのため相対時間を計算
                        let unittime = unittime as f64;
                        let sin_input = (coefficient * frequency * unittime) + (sound.phase as f64);

                        let sample = sound.intensity * sin_input.sin();
                        assert!(sample >= -1.0 && sample <= 1.0);
                        samples.push(sample);
                    }
                }
                EFrequencyItem::Chirp {
                    start_frequency,
                    end_frequency,
                } => {
                    for unittime in 0..samples_count.length {
                        // 振幅と周波数のエンベロープのため相対時間を計算
                        let unittime = unittime as f64;
                        let samples_per_sec = format.samples_per_sec as f64;
                        let mul_factor = {
                            let sample_length = samples_per_sec * (sound.length_sec as f64);
                            let divided = unittime / (sample_length - 1.0);
                            let divided2 = unittime * 0.5;
                            divided * divided2
                        };

                        let frequency = (start_frequency * unittime) + ((end_frequency - start_frequency) * mul_factor);
                        let sin_input = (coefficient * frequency) + (sound.phase as f64);
                        let sample = sound.intensity * sin_input.sin();

                        assert!(sample >= -1.0 && sample <= 1.0);
                        samples.push(sample);
                    }
                }
                EFrequencyItem::Sawtooth { frequency } => {
                    match sound.oscillator_vibrato.as_ref() {
                        Some(vibrato) => {
                            let mut unittime = 0usize;
                            while unittime < samples_count.length {
                                let target_frequency =
                                    vibrato.compute_frequency(*frequency, unittime, format.samples_per_sec as usize);

                                for local_i in 0usize.. {
                                    let rel_time = (local_i as f64) / (format.samples_per_sec as f64);
                                    let rate = rel_time * target_frequency;

                                    let orig_intensity = 1.0 - (2.0 * rate.fract());
                                    samples.push(sound.intensity * orig_intensity);

                                    unittime += 1;
                                    if rate >= 1.0 {
                                        break;
                                    }
                                }
                            }
                        }
                        None => {
                            for unittime in 0..samples_count.length {
                                // 振幅と周波数のエンベロープのため相対時間を計算
                                let relative_time = (unittime as f64) / (format.samples_per_sec as f64);

                                let orig_intensity = 1.0 - (2.0 * (relative_time * frequency).fract());
                                let sample = sound.intensity * orig_intensity;

                                assert!(sample >= -1.0 && sample <= 1.0);
                                samples.push(sample);
                            }
                        }
                    }
                }
            }

            samples
        };

        let mut buffer = vec![];
        buffer.reserve(samples_count.length);
        for unittime in 0..samples_count.length {
            let relative_time = (unittime as f64) / (format.samples_per_sec as f64);

            // ここでdynamic_mul_funcを使って掛け算を行う。
            // もし範囲を超える可能性もあるので、もう一度Clampをかける。
            let intensity_envelop: f64 = sound
                .intensity_control_items
                .iter()
                .map(|v| v.calculate_factor(relative_time, sound))
                .product();

            let input_sample = (samples[unittime] * intensity_envelop).clamp(-1f64, 1f64);
            buffer.push(UniformedSample::from_f64(input_sample));
        }

        // 値を返す。
        Some(Self {
            sound: sound.clone(),
            samples_count,
            buffer,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WaveSound {
    pub format: WaveFormatSetting,
    pub sound_fragments: Vec<SoundFragment>,
}

#[derive(Debug, Clone)]
pub struct OscillatorVibrato {
    pub period_scale_factor: f64,
    pub periodic_frequency: f64,
}

impl OscillatorVibrato {
    pub fn compute_frequency(&self, initial_frequency: f64, sample_i: usize, samples_per_sec: usize) -> f64 {
        let time = (sample_i as f64) / (samples_per_sec as f64);
        initial_frequency + (self.period_scale_factor * (PI2 * self.periodic_frequency * time).sin())
    }
}

impl WaveSound {
    pub fn from_setting(format: &WaveFormatSetting, sound: &WaveSoundSetting) -> Self {
        let p_sound = sound as *const WaveSoundSetting;
        let sounds = unsafe { std::slice::from_raw_parts(p_sound, 1) };

        Self::from_settings(format, sounds)
    }

    pub fn from_settings(format: &WaveFormatSetting, sounds: &[WaveSoundSetting]) -> Self {
        // 今は複数のサウンド波形は一つのバッファーに入れることにする。
        // 後で拡張できればいいだけ。
        let sound_fragments = sounds
            .iter()
            .map(|v| SoundFragment::from_setting(format, v).unwrap())
            .collect_vec();

        WaveSound {
            format: *format,
            sound_fragments,
        }
    }

    pub fn from_builder(builder: WaveSoundBuilder) -> Self {
        // 今は複数のサウンド波形は一つのバッファーに入れることにする。
        // 後で拡張できればいいだけ。
        //
        // `as_ref()`で参照だけを取っているので、中で渡す時にはCloneする。
        let sound_fragments = builder
            .sound_settings
            .iter()
            .map(|v| SoundFragment::from_setting(&builder.format, v).unwrap())
            .collect_vec();

        WaveSound {
            format: builder.format,
            sound_fragments,
        }
    }
}

impl WaveSound {
    pub fn completed_samples_count(&self) -> usize {
        let buffer_end_index = {
            let mut result = 0;
            for fragment in &self.sound_fragments {
                let end_index = fragment.samples_count.end_index();
                if result < end_index {
                    result = end_index;
                }
            }

            result
        };
        buffer_end_index
    }

    ///
    pub fn get_completed_samples(&self) -> Vec<UniformedSample> {
        // bufferの最終サイズを決める。
        let mut buffer = vec![];
        let buffer_end_index = self.completed_samples_count();
        buffer.resize(buffer_end_index, UniformedSample::MIN);

        // そして各fragmentから適切なIndex位置に自分のサンプルを入れる。
        for fragment in &self.sound_fragments {
            let start_i = fragment.samples_count.begin_index;
            for (sample_i, sample) in fragment.buffer.iter().enumerate() {
                let cursor_i = start_i + sample_i;

                //dbg!(buffer[cursor_i], sample);
                buffer[cursor_i] += *sample;
            }
        }

        buffer
    }
}

#[derive(Debug)]
pub struct WaveSoundBuilder {
    pub format: WaveFormatSetting,
    pub sound_settings: Vec<WaveSoundSetting>,
}

impl WaveSoundBuilder {
    pub fn into_build(self) -> WaveSound {
        WaveSound::from_builder(self)
    }
}
