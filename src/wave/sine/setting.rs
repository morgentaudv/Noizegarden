use crate::wave::filter::FilterADSR;

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
    pub const fn to_u32(self) -> u32 {
        match self {
            EBitsPerSample::Bits16 => 16u32,
        }
    }

    /// [`EBitsPerSample`]をバイトサイズに合わせて変換する。
    pub const fn to_byte_size(self) -> usize {
        (self.to_u32() as usize) / 8
    }
}

///
#[derive(Debug, Clone, Copy)]
pub struct WaveFormatSetting {
    pub samples_per_sec: u32,
    #[deprecated = "Deprecated."]
    pub bits_per_sample: EBitsPerSample,
}

#[derive(Debug, Clone, Copy)]
pub enum EFrequencyItem {
    Constant {
        frequency: f64,
    },
    Chirp {
        start_frequency: f64,
        end_frequency: f64,
    },
    Sawtooth {
        frequency: f64,
    },
    Triangle {
        frequency: f64,
    },
    Square {
        duty_rate: f64,
        frequency: f64,
    },
    /// ホワイトノイズを出力する
    WhiteNoise,
    /// ピンクノイズを出力する
    PinkNoise,
    FreqModulation {
        carrier_amp: f64,
        carrier_freq: f64,
        modulator_amp: f64,
        freq_ratio: f64,
        carrier_amp_adsr: Option<WaveSoundADSR>,
        modulator_amp_adsr: Option<WaveSoundADSR>,
    },
}

impl Default for EFrequencyItem {
    fn default() -> Self {
        Self::Constant { frequency: 0.0 }
    }
}

///
#[derive(Debug, Clone, Copy)]
pub struct WaveSoundADSR {
    pub attack_len_second: f64,
    pub decay_len_second: f64,
    pub sustain_intensity: f64,
    pub release_len_second: f64,
    pub gate_len_second: f64,
    pub duration_len_second: f64,
    /// 元となる周波数とADSRの計算によるIntensityを処理して最終的に使う周波数を返す。
    pub process_fn: fn(orig: f64, adsr_intensity: f64) -> f64,
}

impl WaveSoundADSR {
    fn get_samples_len(len_second: f64, samples_per_second: usize) -> usize {
        (len_second * (samples_per_second as f64)).floor() as usize
    }

    pub fn compute(&self, sample_i: usize, samples_per_second: usize) -> f64 {
        FilterADSR {
            attack_sample_len: Self::get_samples_len(self.attack_len_second, samples_per_second),
            decay_sample_len: Self::get_samples_len(self.decay_len_second, samples_per_second),
            sustain_intensity: self.sustain_intensity,
            release_sample_len: Self::get_samples_len(self.release_len_second, samples_per_second),
            gate_sample_len: Self::get_samples_len(self.gate_len_second, samples_per_second),
            duration_sample_len: Self::get_samples_len(self.duration_len_second, samples_per_second),
            process_fn: self.process_fn,
        }
        .compute(sample_i)
    }
}

