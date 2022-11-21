use derive_builder::Builder;
use itertools::Itertools;
use std::{collections::btree_set::Union, f64::consts::PI};

use super::sample::UniformedSample;

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
        }
    }
}

///
#[derive(Default, Debug, Clone, Builder)]
#[builder(default)]
pub struct WaveSoundSetting {
    pub frequency: f32,
    pub phase: f32,
    pub start_sec: f32,
    pub length_sec: f32,
    pub intensity: f64,
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
    fn calc_samples_count(
        samples_per_sec: u32,
        sound_setting: &WaveSoundSetting,
    ) -> CalculatedSamplesCount {
        // サンプル/秒と長さからサンプルの数を計算する。
        // 今はMONOなので1:1でマッチできる。
        //
        // 小数点がある場合には、総サンプルは足りないよりは余ったほうが都合が良いかもしれない。
        let samples = samples_per_sec as f32;
        let begin_index = (samples * sound_setting.start_sec).ceil() as usize;
        let length = (samples * sound_setting.length_sec).ceil() as usize;

        CalculatedSamplesCount {
            begin_index,
            length,
        }
    }

    /// 設定数値から単一サウンドを生成して返す。
    pub fn from_setting(format: &WaveFormatSetting, sound: &WaveSoundSetting) -> Option<Self> {
        // サンプル/秒と長さからサンプルの数を計算する。
        // 今はMONOなので1:1でマッチできる。
        // 小数点がある場合には、総サンプルは足りないよりは余ったほうが都合が良いかもしれない。
        let samples_count = Self::calc_samples_count(format.samples_per_sec, sound);
        //dbg!(samples_count);

        let mut buffer = vec![];
        buffer.reserve(samples_count.length);

        // Sin波形に入れる値はf64として計算する。
        // そしてu32に変換する。最大値は`[2^32 - 1]`である。
        let coefficient = 2.0f64 * PI * (sound.frequency as f64) / (format.samples_per_sec as f64);
        for unittime in 0..samples_count.length {
            let sin_input = coefficient * (unittime as f64) + (sound.phase as f64);
            let sample = sound.intensity * sin_input.sin();
            assert!(sample >= -1.0 && sample <= 1.0);

            // ここでdynamic_mul_funcを使って掛け算を行う。
            // もし範囲を超える可能性もあるので、もう一度Clampをかける。
            let relative_time = (unittime as f64) / (format.samples_per_sec as f64);
            let sample_mul_factor: f64 = sound
                .intensity_control_items
                .iter()
                .map(|v| v.calculate_factor(relative_time, sound))
                .product();

            let input_sample = (sample * sample_mul_factor).clamp(-1f64, 1f64);
            //dbg!(input_sample);

            let uniformed_sample = UniformedSample::from_f64(input_sample);
            buffer.push(uniformed_sample);
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

    pub fn completed_samples_count(&self) -> usize {
        let mut buffer_end_index = {
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
