use std::{io, ops::BitAnd};

use crate::wave::container::WaveContainer;

use super::{
    data::LowWaveDataChunk,
    fact::LowWaveFactChunk,
    fmt::{self, LowWaveFormatHeader, WAV_IMA_ADPCM_BLOCK_SIZE, WAV_IMA_ADPCM_SAMPLES_PER_BLOCK},
    riff::LowWaveRiffHeader,
};

const WAV_IMA_ADPCM_BLOCK_BUFFER_COUNT: usize = (WAV_IMA_ADPCM_SAMPLES_PER_BLOCK >> 1) as usize;

const INDEX_TABLE: [i32; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];

const STEP_SIZE_TABLE: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66, 73, 80, 88, 97, 107,
    118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449, 494, 544, 598, 658, 724, 796, 876, 963,
    1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894,
    6484, 7132, 7845, 8630, 9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794,
    32767,
];

/// @brief IMA-ADPCMの各データブロックのヘッダー情報
#[repr(C)]
#[derive(Debug, Clone)]
struct DataBlock {
    basis_sample_low_byte: u8,
    basis_sample_high_byte: u8,
    step_size_table_index: u8,
    dummy_sample: u8,
}

impl DataBlock {
    const STRUCTURE_SIZE: usize = std::mem::size_of::<DataBlock>();

    fn from_info(basis_sample: i16, step_size_table_index: u8) -> Self {
        Self {
            basis_sample_low_byte: (basis_sample & 0x00FF) as u8,
            basis_sample_high_byte: ((basis_sample >> 8) & 0x00FF) as u8,
            step_size_table_index,
            dummy_sample: 0,
        }
    }

    fn write<T>(&self, writer: &mut T) -> ()
    where
        T: io::Write + io::Seek,
    {
        let mut buffer = [0u8; Self::STRUCTURE_SIZE];
        unsafe {
            std::ptr::write(buffer.as_mut_ptr() as *mut _, (*self).clone());
        }
        writer.write(&buffer).expect("Failed to write LowWaveFactChunk to writer.");
    }
}

/// @brief IMA-ADPCMの各ブロックのヘッダーの後に入れるデータ情報コンテナ
#[repr(C)]
#[derive(Debug, Clone)]
struct DataBlockBuffer {
    // 252 * 2 = 504個。
    buffer: [u8; WAV_IMA_ADPCM_BLOCK_BUFFER_COUNT],
    // bufferのインデックスではなく、差分データのインデックス。
    index: usize,
}

impl Default for DataBlockBuffer {
    fn default() -> Self {
        Self {
            buffer: [0u8; WAV_IMA_ADPCM_BLOCK_BUFFER_COUNT],
            index: 0usize,
        }
    }
}

impl DataBlockBuffer {
    /// 差分データを追加。
    fn add_data(&mut self, data: u8) {
        let buffer_i = self.index >> 1;
        let is_low_bits = self.index % 2 == 0;
        // チェック
        debug_assert!(buffer_i < WAV_IMA_ADPCM_BLOCK_BUFFER_COUNT);

        // 1Byteに2個のサンプル差分情報を保持する。
        // 0bAAAA'BBBBのように…
        let target = &mut self.buffer[buffer_i];
        if is_low_bits {
            *target |= (data & 0xF);
        } else {
            *target |= (data & 0xF) << 4;
        }

        self.index += 1;
    }

    fn write<T>(&self, writer: &mut T) -> ()
    where
        T: io::Write + io::Seek,
    {
        // 書き込んだところだけまで記述する。
        let buffer_length = self.index >> 1;
        let slice = &self.buffer[0..buffer_length];
        if slice.is_empty() {
            return;
        }

        writer.write(&slice).expect("Failed to write buffer to writer.");
    }
}

/// @brief IMA-ADPCM形式に既存WaveContainerを変換して出力するためのもの。
pub struct IMAADPCMWriter<'a> {
    pub source_container: &'a WaveContainer,
}

impl<'a> IMAADPCMWriter<'a> {
    /// @brief `writer`に既存`source_container`をIMA-ADPCM形式に変換して出力する。
    pub fn write<T>(&'a self, writer: &'a mut T) -> ()
    where
        T: io::Write + io::Seek,
    {
        let container = self.source_container;
        if container.channel() > 1 {
            return;
        }

        let format_header = LowWaveFormatHeader::from_builder(fmt::EBuilder::IMA_ADPCM {
            samples_per_sec: container.samples_per_second(),
        });

        // IMA-ADPCMで使うサンプルブロックの数を求める。
        let source_buffer = container.uniformed_sample_buffer();
        let block_size = WAV_IMA_ADPCM_BLOCK_SIZE;
        let samples_per_block = (WAV_IMA_ADPCM_SAMPLES_PER_BLOCK as usize);
        let blocks_count = source_buffer.len() / samples_per_block;

        let chunk_size = (block_size as u32) * (blocks_count as u32);
        let data_chunk = LowWaveDataChunk::from_chunk_size(chunk_size);

        // Write RIFF
        {
            let riff_header = LowWaveRiffHeader::from_data_chunk_with_ima_adpcm(&data_chunk);
            riff_header.write(writer);
        }

        // Write FMT
        {
            format_header.write(writer);

            // 拡張チャンクのサイズ指定。
            // extra_size (2bytes)
            let buffer = unsafe {
                let mut buffer = [0u8; 2];
                let extra_size = 2u16;
                std::ptr::write(buffer.as_mut_ptr() as *mut _, extra_size);
                buffer
            };
            writer.write(&buffer).expect("Failed to write extra_size to writer.");

            // IMA_ADPCMの使用準拠でsamples_per_blockの記入。(2bytes)
            let buffer = unsafe {
                let mut buffer = [0u8; 2];
                std::ptr::write(buffer.as_mut_ptr() as *mut _, WAV_IMA_ADPCM_SAMPLES_PER_BLOCK);
                buffer
            };
            writer.write(&buffer).expect("Failed to write samples_per_block to writer.");
        }

        // Write FACT
        {
            let sample_length = (WAV_IMA_ADPCM_SAMPLES_PER_BLOCK as u32 * (blocks_count as u32)) + 1;
            let fact_header = LowWaveFactChunk::from_sample_length(sample_length);
            fact_header.write(writer);
        }

        // Write DATA
        {
            data_chunk.write(writer);
        }

        // 既存BufferをADPCMバッファに変換して記録する。
        let mut step_size_table_i = 0usize; // step_size_table_i := index.
        let mut basis_sample = 0i16; // basis_sample := sp
        let mut data_block = DataBlock::from_info(basis_sample, step_size_table_i as u8);

        for block_i in 0..blocks_count {
            let sample_i_offset = block_i * samples_per_block;
            let mut data_buffer = DataBlockBuffer::default();

            // local_si := local sample index.
            for local_si in 0..samples_per_block {
                let sample_i = local_si + sample_i_offset;
                let sample = source_buffer[sample_i].to_16bits(); // sample := s.

                // もしBlockのローカルインデックスが最初なら、Blockのヘッダーを初期化する。
                if local_si == 0 {
                    basis_sample = sample;
                    data_block = DataBlock::from_info(basis_sample, step_size_table_i as u8);
                    continue;
                }

                // ここからはbasis_sampleといろいろと使ってAdaptiveな差分を求めて記録する。
                // abs_d := d.
                let step_size = STEP_SIZE_TABLE[step_size_table_i];
                let (mut c, mut abs_d) = {
                    let d = (sample as i32) - (basis_sample as i32);
                    if d < 0 {
                        (0b1000u8, d.abs())
                    } else {
                        (0b0000u8, d)
                    }
                };

                // 圧縮フェーズ
                if abs_d >= step_size {
                    c |= 0x04u8;
                    abs_d -= step_size;
                }
                if abs_d >= (step_size >> 1) {
                    c |= 0x02u8;
                    abs_d -= step_size >> 1;
                }
                if abs_d >= (step_size >> 2) {
                    c |= 0x01u8;
                }

                // 伸張フェーズ。
                let dp = {
                    let mut v = step_size >> 3;
                    if c.bitand(0x1) > 0 {
                        v += step_size >> 2;
                    }
                    if c.bitand(0x2) > 0 {
                        v += step_size >> 1;
                    }
                    if c.bitand(0x4) > 0 {
                        v += step_size;
                    }
                    v
                };

                // basis_sampleを再指定。
                basis_sample = {
                    let new_sample = if c.bitand(0x8) > 0 {
                        basis_sample as i32 - dp
                    } else {
                        basis_sample as i32 + dp
                    };
                    new_sample.clamp(i16::MIN as i32, i16::MAX as i32) as i16
                };

                // step_size_table_iの再指定。
                step_size_table_i = ((step_size_table_i as i32) + INDEX_TABLE[c as usize]).clamp(0, 88) as usize;

                // バッファーに書き込み。
                data_buffer.add_data(c);
            }

            // 書き込む。
            data_block.write(writer);
            data_buffer.write(writer);
        }
    }
}
