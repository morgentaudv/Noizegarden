use itertools::Itertools;
use wav::{
    data::LowWaveDataChunk,
    fact::LowWaveFactChunk,
    fmt::{self, LowWaveFormatHeader},
    riff::LowWaveRiffHeader,
};

use super::{
    sample::UniformedSample,
    stretch::time::{TimeStretcherBufferSetting, TimeStretcherBuilder},
};
use crate::wave::container::wav::bext::LowWaveBextHeader;
use crate::wave::container::wav::junk::LowWaveJunkHeader;
use crate::wave::container::wav::qlty::LowWaveQualityHeader;
use crate::wave::container::wav::try_read_wave_header_id_str;
use num_traits::Zero;
use std::io;
use std::ops::BitAnd;

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
    /// 放送業界用Chunk。
    bext: Option<LowWaveBextHeader>,
    /// Option用
    qlty: Option<LowWaveQualityHeader>,
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
        const MINIMUM_SIZE: usize = size_of::<LowWaveRiffHeader>()
            + size_of::<LowWaveFormatHeader>()
            + size_of::<LowWaveFactChunk>()
            + size_of::<LowWaveDataChunk>();

        // readerの大きさを計算して判定を行う。
        {
            let reader_length = reader.seek(io::SeekFrom::End(0)).expect("Failed to seek reader.") as usize;
            reader.rewind().expect("Failed to rewind reader.");
            if MINIMUM_SIZE > reader_length {
                // Chunkのサイズが足りなければ、そもそも読み込む必要はない。
                return None;
            }
        }

        // 情報を取得する。
        let mut wave_riff_header = None;
        let mut wave_fmt_header = None;
        let mut wave_fact_chunk = None;
        let mut wave_bext_header = None;
        let mut wave_qlty_header = None;
        loop {
            let id = try_read_wave_header_id_str(reader);
            match id.as_str() {
                "RIFF" => {
                    wave_riff_header =
                        Some(LowWaveRiffHeader::from_bufread(reader).expect("Failed to get riff header."));
                }
                "fmt " => {
                    wave_fmt_header =
                        Some(LowWaveFormatHeader::from_bufread(reader).expect("Failed to get fmt header."));
                }
                "fact" => {
                    wave_fact_chunk = Some(LowWaveFactChunk::from_bufread(reader).expect("Failed to get fact chunk."))
                }
                "bext" => {
                    // 25-01-08 放送業界(EBC)で決めたWav拡張ヘッダーらしい。
                    // このプログラムではまだ活用しない。
                    // bext, 4bytesで次に来るチャンクの大きさ、そしてチャンクのデータがくる。
                    wave_bext_header =
                        Some(LowWaveBextHeader::from_bufread(reader).expect("Failed to get bext chunk."));
                }
                "junk" => {
                    // 25-01-08
                    let _junk_header =
                        Some(LowWaveJunkHeader::from_bufread(reader).expect("Failed to get junk chunk."));
                }
                "qlty" => {
                    // 25-01-09
                    wave_qlty_header =
                        Some(LowWaveQualityHeader::from_bufread(reader).expect("Failed to get qlty chunk."));
                }
                "data" => {
                    break; // data以降はデータしか含まないはず。
                }
                _ => unreachable!("Unexpected header ID."),
            }
        }

        let wave_data_chunk = LowWaveDataChunk::from_bufread(reader).expect("Failed to get data chunk");
        debug_assert!(wave_fmt_header.is_some());

        let wave_fmt_header = wave_fmt_header.unwrap();

        // 最後に実際データが入っているバッファーを読み取る。
        let mut buffer = vec![];
        reader.read_to_end(&mut buffer).expect("Failed to read buffer.");

        // bufferの各ブロックから`UniformedSample`に変換する。
        let unit_block_size = wave_fmt_header.unit_block_size();
        let bits_per_sample = wave_fmt_header.bits_per_sample as usize;
        let uniformed_buffer = match bits_per_sample {
            16 => {
                // LPCM 16bits
                //
                // 16Bitsは [-32768, 32768)の範囲を持つ。
                // 読み取ったバッファーからブロックサイズと量子化ビットサイズに合わせてsliceに変換する。
                let p_buffer = buffer.as_ptr() as *const i16;
                let data_count = (wave_data_chunk.data_chunk_size as usize) / unit_block_size;
                let buffer_slice = unsafe { std::slice::from_raw_parts(p_buffer, data_count) };

                buffer_slice.iter().map(|&v| UniformedSample::from_16bits(v)).collect_vec()
            }
            24 => {
                // LPCM 24bits
                //
                // −8,388,608 to +8,388,607を持つ。
                // 一つのサンプルが3Bytesパッキングされているので、慎重に読み取る。
                let data_count = (wave_data_chunk.data_chunk_size as usize) / unit_block_size;

                // ここ最適化できそうだけど、今は愚直な方法で。
                // 実は元バッファーのSliceを渡して変換することもできるという。
                let mut raw_buffer = vec![];
                raw_buffer.reserve(data_count);
                for i in 0..data_count {
                    let start_i = 3 * i;
                    let raw_sample = &buffer[start_i..(start_i + 3)];
                    // データの入り方がBig Endianになっているので、sample[2]の一番前のビットが1なら負の数扱いにする。
                    // この辺ちょっとめんどくさい。
                    let offset = if raw_sample[2].bitand(0b10000000).is_zero() { 0x00 } else { 0xFF };
                    let raw_sample = [offset, raw_sample[2], raw_sample[1], raw_sample[0]];
                    let raw_sample = i32::from_be_bytes(raw_sample);

                    // チェック
                    debug_assert!(raw_sample >= -8_388_608);
                    debug_assert!(raw_sample < 8_388_608);
                    // 3Bytesすすむ。
                    raw_buffer.push(raw_sample);
                }

                raw_buffer
                    .into_iter()
                    .map(|v| UniformedSample::from_i32_as_24bit(v))
                    .collect_vec()
            }
            _ => unreachable!("Unexpected branch"),
        };

        Some(WaveContainer {
            riff: wave_riff_header.unwrap(),
            fmt: wave_fmt_header,
            bext: wave_bext_header,
            qlty: wave_qlty_header,
            fact: wave_fact_chunk,
            data: wave_data_chunk,
            uniformed_buffer,
        })
    }

    ///
    pub fn from_uniformed_sample_buffer(original: &Self, uniformed_buffer: Vec<UniformedSample>) -> Self {
        Self {
            riff: original.riff.clone(),
            fmt: original.fmt.clone(),
            bext: original.bext.clone(),
            qlty: original.qlty.clone(),
            fact: original.fact.clone(),
            data: original.data.clone(),
            uniformed_buffer,
        }
    }

    /// [`WaveContainer`]の情報を[`io::Write`]ストリームに書き込む。
    /// `writer`は[`io::Write`]と[`io::Seek`]を実装していること。
    ///
    /// `writer`のflush動作などは行わない。
    pub fn write<T>(&self, writer: &mut T) -> ()
    where
        T: io::Write + io::Seek,
    {
        self.riff.write(writer);
        self.fmt.write(writer);

        if self.bext.is_some() {
            self.bext.as_ref().unwrap().write(writer);
        }

        if self.qlty.is_some() {
            self.qlty.as_ref().unwrap().write(writer);
        }

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
                        let p_buffer = converted_buffer.as_ptr();
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
        assert_eq!(self.fmt.channel, 1);
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
    pub fn build_mono(&self, uniformed_samples: Vec<UniformedSample>) -> Option<WaveContainer> {
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
            channels: 1,
        };
        let format_header = LowWaveFormatHeader::from_builder(builder);
        let data_chunk_size = (format_header.unit_block_size() * uniformed_samples.len()) as u32;
        let data_chunk = LowWaveDataChunk::from_chunk_size(data_chunk_size);
        let riff_header = LowWaveRiffHeader::from_data_chunk(&data_chunk);

        Some(WaveContainer {
            riff: riff_header,
            fmt: format_header,
            bext: None,
            qlty: None,
            fact: None,
            data: data_chunk,
            uniformed_buffer: uniformed_samples,
        })
    }

    /// [u-law](https://en.wikipedia.org/wiki/%CE%9C-law_algorithm)のPCMU形式のコンテナに変換する。
    pub fn from_container_to_ulaw_mono(container: &WaveContainer) -> Option<WaveContainer> {
        if container.channel() > 1 {
            return None;
        }

        let src_container = container.uniformed_sample_buffer();
        let format_header = LowWaveFormatHeader::from_builder(fmt::EBuilder::Pcmu);
        let data_chunk_size = (format_header.unit_block_size() * src_container.len()) as u32;
        let data_chunk = LowWaveDataChunk::from_chunk_size(data_chunk_size + 2);
        let riff_header = LowWaveRiffHeader::from_data_chunk(&data_chunk);

        let dst_container = {
            let shrink_rate = (8000.0 / (container.samples_per_second() as f64)).recip();
            if shrink_rate == 1.0 {
                // F_sがそのままなのでOK
                src_container.to_owned()
            } else {
                // TimeStretcherを使ってResamplingする。（精度はちゃんとしたアルゴリズムに比べたら落ちる）
                let original_fs = container.samples_per_second();
                let template_size = (original_fs as f64 * 0.01) as usize;
                let p_min = (original_fs as f64 * 0.005) as usize;
                let p_max = (original_fs as f64 * 0.02) as usize;
                let setting = TimeStretcherBufferSetting { buffer: src_container };

                TimeStretcherBuilder::default()
                    .template_size(template_size)
                    .shrink_rate(shrink_rate)
                    .sample_period_min(p_min)
                    .sample_period_length(p_max - p_min)
                    .build()
                    .unwrap()
                    .process_with_buffer(&setting)?
            }
        };

        Some(WaveContainer {
            riff: riff_header,
            fmt: format_header,
            bext: None,
            qlty: None,
            fact: None,
            data: data_chunk,
            uniformed_buffer: dst_container,
        })
    }

    pub fn build_stereo(&self, left: Vec<UniformedSample>, right: Vec<UniformedSample>) -> Option<WaveContainer> {
        if self.bits_per_sample != 8 && self.bits_per_sample != 16 {
            return None;
        }
        if self.samples_per_sec == 0 {
            return None;
        }
        assert_eq!(left.len(), right.len());

        // ローレベルのヘッダーの情報などを作る。
        let builder = fmt::EBuilder::Normal {
            samples_per_sec: self.samples_per_sec,
            bits_per_sample: self.bits_per_sample,
            channels: 2,
        };
        let format_header = LowWaveFormatHeader::from_builder(builder);
        let data_chunk_size = (format_header.unit_block_size() * (left.len() + right.len())) as u32;
        let data_chunk = LowWaveDataChunk::from_chunk_size(data_chunk_size);
        let riff_header = LowWaveRiffHeader::from_data_chunk(&data_chunk);

        // [`WaveContainer::uniformed_buffer`]はStereoなのでleft→rightのようにする。
        let mut uniformed_buffer = vec![];
        uniformed_buffer.reserve(left.len() + right.len());
        for (l_s, r_s) in left.iter().zip(right.iter()) {
            uniformed_buffer.push(*l_s);
            uniformed_buffer.push(*r_s);
        }
        Some(WaveContainer {
            riff: riff_header,
            fmt: format_header,
            bext: None,
            qlty: None,
            fact: None,
            data: data_chunk,
            uniformed_buffer,
        })
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
