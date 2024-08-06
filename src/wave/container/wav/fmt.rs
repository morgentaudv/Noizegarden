use std::io;

use crate::wave::setting::WaveSound;

pub const WAV_DATATYPE_LPCM: u16 = 1;
pub const WAV_DATATYPE_PCMU: u16 = 7;

/// WAVファイルのチャンクフォーマットタイプを表す。
pub(crate) enum EWavFormatType {
    Unknown,
    LPCM,
    PCMU,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub(crate) struct LowWaveFormatHeader {
    /// `"fmt "`と同様
    fmt_chunk_id: [u8; 4],
    /// [`LowWaveFormatHeader::CHUNK_SIZE`]と同様
    fmt_chunk_size: u32,
    wave_format_type: u16,
    pub channel: u16,
    pub samples_per_sec: u32,
    ///
    bytes_per_sec: u32,
    /// チャンネルを含む各サンプルの総サイズ。
    /// もしチャンネルを分離した本当の各サンプルのサイズが取得したい場合には
    /// [`LowWaveFormatHeader::unit_block_size`]メソッドを使う。
    block_size: u16,
    pub bits_per_sample: u16,
}

const_assert_eq!(LowWaveFormatHeader::STRUCTURE_SIZE, 24usize);

#[derive(Debug, Clone, Copy)]
pub(crate) enum EBuilder {
    Normal { samples_per_sec: u32, bits_per_sample: u16 },
    PCMU,
}

impl LowWaveFormatHeader {
    const STRUCTURE_SIZE: usize = std::mem::size_of::<LowWaveFormatHeader>();
    const NORMAL_CHUNK_SIZE: u32 = 16;
    /// PCMU(u-law)のときのChunkSize。
    const PCMU_CHUNK_SIZE: u32 = 18;
    const ID_SPECIFIER: [u8; 4] = ['f' as u8, 'm' as u8, 't' as u8, ' ' as u8];

    pub(crate) fn from_builder(setting: EBuilder) -> Self {
        match setting {
            EBuilder::Normal {
                samples_per_sec,
                bits_per_sample,
            } => {
                let block_size = (bits_per_sample >> 3) as u16;
                let bytes_per_sec = (block_size as u32) * samples_per_sec;
                Self {
                    fmt_chunk_id: Self::ID_SPECIFIER,
                    fmt_chunk_size: Self::NORMAL_CHUNK_SIZE,
                    wave_format_type: WAV_DATATYPE_LPCM,
                    channel: 1,
                    samples_per_sec,
                    bytes_per_sec,
                    block_size,
                    bits_per_sample,
                }
            }
            EBuilder::PCMU => Self {
                fmt_chunk_id: Self::ID_SPECIFIER,
                fmt_chunk_size: Self::PCMU_CHUNK_SIZE,
                wave_format_type: WAV_DATATYPE_PCMU,
                channel: 1,
                samples_per_sec: 8000,
                bytes_per_sec: 8000,
                block_size: 1,
                bits_per_sample: 8,
            },
        }
    }

    /// [`WaveSound`]から[`LowWaveFormatHeader`]を生成する。
    pub fn from_wave_sound(sound: &WaveSound) -> Self {
        let channel = 1;
        let unit_block_size = sound.format.bits_per_sample.to_byte_size();
        let block_size = (unit_block_size as u16) * channel;

        Self {
            fmt_chunk_id: Self::ID_SPECIFIER,
            fmt_chunk_size: Self::NORMAL_CHUNK_SIZE,
            wave_format_type: WAV_DATATYPE_LPCM,
            channel,
            samples_per_sec: sound.format.samples_per_sec,
            bytes_per_sec: (block_size as u32) * sound.format.samples_per_sec,
            block_size,
            bits_per_sample: sound.format.bits_per_sample.to_u32() as u16,
        }
    }

    /// `io::Read + io::Seek`から`Self`の情報を取得して作る。
    pub fn from_bufread<T>(reader: &mut T) -> Option<Self>
    where
        T: io::Read + io::Seek,
    {
        let mut buffer = [0u8; Self::STRUCTURE_SIZE];
        reader.read(&mut buffer[..]).expect("Failed to read fmt header.");

        // ヘッダーがちゃんとしているかを確認してから返す。失敗したらそのまま終了。
        let maybe_header: Self = unsafe { std::ptr::read(buffer.as_ptr() as *const _) };

        // fmt_idの確認。
        {
            let id = std::str::from_utf8(&maybe_header.fmt_chunk_id).unwrap();
            assert!(id == "fmt ");
        }
        // fmt_chunk_sizeの確認。
        {
            let maybe_size = maybe_header.fmt_chunk_size;
            assert!(maybe_size == Self::NORMAL_CHUNK_SIZE || maybe_size == Self::PCMU_CHUNK_SIZE);
        }

        Some(maybe_header)
    }

    /// [`LowWaveFormatHeader`]の情報を[`std::io::Write`]ストリームに書き込む。
    /// `writer`は[`std::io::Write`]と[`std::io::Seek`]を実装していること。
    pub fn write<T>(&self, writer: &mut T)
    where
        T: io::Write + io::Seek,
    {
        let mut buffer = [0u8; Self::STRUCTURE_SIZE];
        unsafe {
            std::ptr::write(buffer.as_mut_ptr() as *mut _, (*self).clone());
        }
        writer.write(&buffer).expect("Failed to write LowWaveFormatHeader to writer.");

        if self.fmt_chunk_size > 16 {
            // 拡張チャンクのサイズ指定。0Bytes
            let buffer = [0u8; 2];
            writer.write(&buffer).expect("Failed to write LowWaveFormatHeader to writer.");
        }
    }

    /// １個のチャンネルのブロックサイズを返す。
    pub fn unit_block_size(&self) -> usize {
        let block_size = self.block_size as usize;
        block_size / (self.channel as usize)
    }

    pub fn format_type(&self) -> EWavFormatType {
        match self.wave_format_type {
            WAV_DATATYPE_LPCM => EWavFormatType::LPCM,
            WAV_DATATYPE_PCMU => EWavFormatType::PCMU,
            _ => EWavFormatType::Unknown,
        }
    }
}
