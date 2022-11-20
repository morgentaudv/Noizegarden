use derive_builder::Builder;
use itertools::Itertools;
use std::f64::consts::PI;

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

#[derive(Debug, Clone, Copy)]
pub struct WaveFormatSetting {
    pub samples_per_sec: u32,
    pub bits_per_sample: EBitsPerSample,
}

#[derive(Default, Debug, Clone, Copy, Builder)]
#[builder(default)]
pub struct WaveSoundSetting {
    pub frequency: u32,
    pub start_sec: f32,
    pub length_sec: f32,
    pub intensity: f64,
}

#[derive(Debug, Clone)]
pub struct WaveSound {
    pub format: WaveFormatSetting,
    //pub sound: WaveSoundSetting,
    pub buffer: Vec<UniformedSample>,
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

impl WaveSound {
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

    pub fn from_setting(format: &WaveFormatSetting, sound: &WaveSoundSetting) -> Self {
        let p_sound = sound as *const WaveSoundSetting;
        let sounds = unsafe { std::slice::from_raw_parts(p_sound, 1) };

        Self::from_settings(format, sounds)
    }

    pub fn from_settings(format: &WaveFormatSetting, sounds: &[WaveSoundSetting]) -> Self {
        let mut buffer = vec![];

        // 今は複数のサウンド波形は一つのバッファーに入れることにする。
        // 後で拡張できればいいだけ。
        let count_infos = sounds
            .iter()
            .map(|v| Self::calc_samples_count(format.samples_per_sec, v))
            .collect_vec();
        dbg!(&count_infos);

        // count_infosから最終end_indexを求める。
        // 一番大きいのが最後。
        let last_end_index = {
            let mut result = 0usize;
            for count_info in &count_infos {
                let end_index = count_info.end_index();
                if result < end_index {
                    result = end_index;
                }
            }
            result
        };

        // サンプル/秒と長さからサンプルの数を計算する。
        // 今はMONOなので1:1でマッチできる。
        //
        // 小数点がある場合には、総サンプルは足りないよりは余ったほうが都合が良いかもしれない。
        buffer.resize(last_end_index, UniformedSample::MIN);

        // Sin波形に入れる値はf64として計算する。
        // そしてu32に変換する。最大値は`[2^32 - 1]`である。
        for (count_info, sound) in count_infos.into_iter().zip(sounds) {
            let coefficient =
                2.0f64 * PI * (sound.frequency as f64) / (format.samples_per_sec as f64);

            for unittime in 0..count_info.length {
                let sin_input = coefficient * (unittime as f64);
                let sample = sound.intensity * sin_input.sin();
                assert!(sample >= -1.0 && sample <= 1.0);

                let uniformed_sample = UniformedSample::from_f64(sample);
                let input_index = count_info.begin_index + unittime;
                buffer[input_index] = uniformed_sample;
            }
        }

        WaveSound {
            format: *format,
            //sound: *sound,
            buffer,
        }
    }
}
