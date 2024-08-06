use itertools::Itertools;
use num_traits::FromPrimitive;
use wav::{
    data::LowWaveDataChunk,
    fact::LowWaveFactChunk,
    fmt::{self, LowWaveFormatHeader},
    riff::LowWaveRiffHeader,
};

use super::{sample::UniformedSample, setting::WaveSound};
use std::{io, mem};

pub mod wav;

// ----------------------------------------------------------------------------
//
// LOW-LEVEL STRUCTURES
//
// ----------------------------------------------------------------------------

/// 音源の情報を保持するコンテナ。
/// wavファイルからの読み込みやwavファイルへの書き込み、その他簡単なフィルタリング機能ができる。
#[derive(Debug)]
pub struct WaveContainer {
    riff: LowWaveRiffHeader,
    fmt: LowWaveFormatHeader,
    /// 非PCM形式のwaveの場合、`LowWaveFactChunk`が存在する。
    fact: Option<LowWaveFactChunk>,
    data: LowWaveDataChunk,
    /// 音源のバッファを平準化して保持する。
    uniformed_buffer: Vec<UniformedSample>,
}

impl WaveContainer {
    ///
    pub fn from_bufread<T>(reader: &mut T) -> Option<Self>
    where
        T: io::Read + io::Seek,
    {
        const MIMINUM_SIZE: usize = mem::size_of::<LowWaveRiffHeader>()
            + mem::size_of::<LowWaveFormatHeader>()
            + mem::size_of::<LowWaveFactChunk>()
            + mem::size_of::<LowWaveDataChunk>();

        // readerの大きさを計算して判定を行う。
        {
            let reader_length = reader.seek(io::SeekFrom::End(0)).expect("Failed to seek reader.") as usize;
            reader.rewind().expect("Failed to rewind reader.");
            if MIMINUM_SIZE > reader_length {
                // Chunkのサイズが足りなければ、そもそも読み込む必要はない。
                return None;
            }
        }

        // 情報を取得する。
        let wave_riff_header = LowWaveRiffHeader::from_bufread(reader).expect("Failed to get riff header.");
        let wave_fmt_header = LowWaveFormatHeader::from_bufread(reader).expect("Failed to get fmt header.");
        let wave_fact_chunk = {
            if LowWaveFactChunk::can_be_chunk(reader) {
                Some(LowWaveFactChunk::from_bufread(reader).expect("Failed to get fact chunk."))
            } else {
                None
            }
        };
        let wave_data_chunk = LowWaveDataChunk::from_bufread(reader).expect("Failed to get data chunk");
        let buffer_size = wave_data_chunk.data_chunk_size / (wave_fmt_header.channel as u32);

        // 最後に実際データが入っているバッファーを読み取る。
        let mut buffer = vec![];
        reader.read_to_end(&mut buffer).expect("Failed to read buffer.");
        assert!(buffer.len() == (buffer_size as usize));

        // bufferの各ブロックから`UniformedSample`に変換する。
        let unit_block_size = wave_fmt_header.unit_block_size();
        let bits_per_sample = wave_fmt_header.bits_per_sample as usize;
        assert!(bits_per_sample == 16);

        // 今の量子化は16Bitsしか対応しない。
        // 16Bitsは [-32768, 32768)の範囲を持つ。
        let uniformed_buffer: Vec<UniformedSample> = {
            // 読み取ったバッファーからブロックサイズと量子化ビットサイズに合わせてsliceに変換する。
            let p_buffer = buffer.as_ptr() as *const i16;
            let data_count = (wave_data_chunk.data_chunk_size as usize) / unit_block_size;
            let buffer_slice = unsafe { std::slice::from_raw_parts(p_buffer, data_count) };

            let converted_buffer = buffer_slice.iter().map(|&v| UniformedSample::from_16bits(v)).collect();
            converted_buffer
        };

        Some(WaveContainer {
            riff: wave_riff_header,
            fmt: wave_fmt_header,
            fact: wave_fact_chunk,
            data: wave_data_chunk,
            //raw_buffer: buffer,
            uniformed_buffer,
        })
    }

    /// [`WaveSound`]から[`WaveContainer`]を生成します。
    pub fn from_wavesound(sound: &WaveSound) -> Option<Self> {
        let data = LowWaveDataChunk::from_wave_sound(sound);
        let riff = LowWaveRiffHeader::from_data_chunk(&data);
        let fmt = LowWaveFormatHeader::from_wave_sound(sound);

        // まず、WaveSoundから各WaveFragmentを収集して単一のバッファーを作る必要がある。
        let uniformed_buffer = sound.get_completed_samples();

        Some(Self {
            riff,
            fmt,
            fact: None,
            data,
            uniformed_buffer,
        })
    }

    ///
    pub fn from_uniformed_sample_buffer(original: &Self, uniformed_buffer: Vec<UniformedSample>) -> Self {
        Self {
            riff: original.riff.clone(),
            fmt: original.fmt.clone(),
            fact: original.fact.clone(),
            data: original.data.clone(),
            uniformed_buffer,
        }
    }

    /// [`WaveContainer`]の情報を[`std::io::Write`]ストリームに書き込む。
    /// `writer`は[`std::io::Write`]と[`std::io::Seek`]を実装していること。
    ///
    /// `writer`のflush動作などは行わない。
    pub fn write<T>(&self, writer: &mut T) -> ()
    where
        T: io::Write + io::Seek,
    {
        self.riff.write(writer);
        self.fmt.write(writer);
        if self.fact.is_some() {
            self.fact.as_ref().unwrap().write(writer);
        }

        self.data.write(writer);

        // `unit_block_size`は各ユニkットブロックのメモリ空間を、
        // `bits_per_sample`は`UniformSample`からどのように数値に変換するかを表す。
        let unit_block_size = self.unit_block_size();
        // そしてバッファーから量子化ビットとブロックサイズに合わせて別リストに変換し書き込ませる。
        let bits_per_sample = self.bits_per_sample();
        match self.fmt.format_type() {
            fmt::EWavFormatType::Unknown => unreachable!(),
            fmt::EWavFormatType::LPCM => {
                if bits_per_sample == 16 {
                    assert_eq!(unit_block_size, 2);

                    let converted_buffer: Vec<i16> = { self.uniformed_buffer.iter().map(|v| v.to_16bits()).collect() };
                    let converted_buffer_slice = unsafe {
                        let p_buffer = converted_buffer.as_ptr() as *const u8;
                        std::slice::from_raw_parts(p_buffer, converted_buffer.len() * 2)
                    };

                    writer
                        .write(&converted_buffer_slice)
                        .expect("Failed to write Buffer to writer.");
                } else if bits_per_sample == 8 {
                    assert_eq!(unit_block_size, 1);

                    let converted_buffer: Vec<u8> =
                        { self.uniformed_buffer.iter().map(|v| v.to_unsigned_8bits()).collect() };
                    let converted_buffer_slice = unsafe {
                        let p_buffer = converted_buffer.as_ptr() as *const u8;
                        std::slice::from_raw_parts(p_buffer, converted_buffer.len())
                    };

                    writer
                        .write(&converted_buffer_slice)
                        .expect("Failed to write Buffer to writer.");
                }
            }
            fmt::EWavFormatType::PCMU => {
                let converted_buffer = { self.uniformed_buffer.iter().map(|v| v.to_ulaw_8bits()).collect_vec() };
                let converted_buffer_slice = unsafe {
                    let p_buffer = converted_buffer.as_ptr() as *const u8;
                    std::slice::from_raw_parts(p_buffer, converted_buffer.len())
                };

                writer
                    .write(&converted_buffer_slice)
                    .expect("Failed to write Buffer to writer.");
            }
        }
    }
}

impl WaveContainer {
    /// １個のチャンネルのブロックサイズを返す。
    pub fn unit_block_size(&self) -> usize {
        self.fmt.unit_block_size()
    }

    /// 各サンプルに適用する量子化ビットを返す。
    pub fn bits_per_sample(&self) -> u32 {
        self.fmt.bits_per_sample as u32
    }

    /// サウンドの秒ごとのサンプル数を返す。
    pub fn samples_per_second(&self) -> u32 {
        self.fmt.samples_per_sec
    }

    /// サウンドの全体長さを秒数で返す。
    pub fn sound_length(&self) -> f64 {
        let items_per_sec = (self.fmt.samples_per_sec as usize) * (self.fmt.channel as usize);
        let sound_length = (self.uniformed_buffer.len() as f64) / (items_per_sec as f64);
        sound_length
    }

    /// サウンドのチャンネル数を返す。
    pub fn channel(&self) -> u32 {
        self.fmt.channel as u32
    }

    /// `time`から一番近い適切なサンプルを返す。
    pub fn uniform_sample_of_f32(&self, time: f32) -> Option<UniformedSample> {
        self.uniform_sample_of_f64(time as f64)
    }

    /// `time`から一番近い適切なサンプルを返す。
    pub fn uniform_sample_of_f64(&self, time: f64) -> Option<UniformedSample> {
        match self.calculate_sample_index_of_time(time) {
            Some(sample_i) => Some(self.uniformed_sample_buffer()[sample_i]),
            None => None,
        }
    }

    /// サンプルが入っているバッファーのSliceを貸す形で返す。
    pub fn uniformed_sample_buffer(&self) -> &'_ [UniformedSample] {
        &self.uniformed_buffer
    }

    ///
    pub(crate) fn calculate_sample_index_of_time(&self, time: f64) -> Option<usize> {
        // 今はチャンネルをMONOに限定する。
        assert!(self.fmt.channel == 1);
        if time >= self.sound_length() {
            return None;
        }

        Some(((self.samples_per_second() as f64) * time).floor() as usize)
    }
}

// ----------------------------------------------------------------------------
//
// BUILDER
//
// ----------------------------------------------------------------------------

pub struct WaveBuilder {
    pub samples_per_sec: u32,
    pub bits_per_sample: u16,
}

impl WaveBuilder {
    pub fn build_container(&self, uniformed_samples: Vec<UniformedSample>) -> Option<WaveContainer> {
        if self.bits_per_sample != 8 && self.bits_per_sample != 16 {
            return None;
        }
        if self.samples_per_sec == 0 {
            return None;
        }

        // 今はMONO、PCMで固定する。
        // ローレベルのヘッダーの情報などを作る。
        let builder = fmt::EBuilder::Normal {
            samples_per_sec: self.samples_per_sec,
            bits_per_sample: self.bits_per_sample,
        };
        let format_header = LowWaveFormatHeader::from_builder(builder);
        let data_chunk_size = (format_header.unit_block_size() * uniformed_samples.len()) as u32;
        let data_chunk = LowWaveDataChunk::from_chunk_size(data_chunk_size);
        let riff_header = LowWaveRiffHeader::from_data_chunk(&data_chunk);

        Some(WaveContainer {
            riff: riff_header,
            fmt: format_header,
            fact: None,
            data: data_chunk,
            uniformed_buffer: uniformed_samples,
        })
    }

    /// [u-law](https://en.wikipedia.org/wiki/%CE%9C-law_algorithm)のPCMU形式のコンテナに変換する。
    pub fn from_container_to_ulaw(container: &WaveContainer) -> Option<WaveContainer> {
        if container.channel() > 1 {
            return None;
        }

        let src_container = container.uniformed_sample_buffer();
        let format_header = LowWaveFormatHeader::from_builder(fmt::EBuilder::PCMU);
        let data_chunk_size = (format_header.unit_block_size() * src_container.len()) as u32;
        let data_chunk = LowWaveDataChunk::from_chunk_size(data_chunk_size + 2);
        let riff_header = LowWaveRiffHeader::from_data_chunk(&data_chunk);
        let dst_container = src_container.iter().map(|v| v.into_ulaw_uniform_sample()).collect_vec();

        Some(WaveContainer {
            riff: riff_header,
            fmt: format_header,
            fact: None,
            data: data_chunk,
            uniformed_buffer: src_container.to_owned(),
        })
    }
}
