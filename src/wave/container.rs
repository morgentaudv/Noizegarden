use super::{sample::UniformedSample, setting::WaveSound};
use std::{io, mem};

///
#[repr(C)]
#[derive(Debug, Clone)]
struct WaveRiffHeader {
    riff_chunk_id: [u8; 4],
    riff_chunk_size: u32,
    file_format_type: [u8; 4],
}

const_assert_eq!(std::mem::size_of::<WaveRiffHeader>(), 12usize);

impl WaveRiffHeader {
    const STRUCTURE_SIZE: usize = std::mem::size_of::<WaveRiffHeader>();
    const CHUNK_MINIMUM_SIZE: u32 = 48;
    const ID_SPECIFIER: [u8; 4] = ['R' as u8, 'I' as u8, 'F' as u8, 'F' as u8];
    const TYPE_SPECIFIER: [u8; 4] = ['W' as u8, 'A' as u8, 'V' as u8, 'E' as u8];

    /// [`WaveDataChunk`]の設定から[`WaveRiffHeader`]を生成する。
    pub fn from_data_chunk(data: &WaveDataChunk) -> Self {
        Self {
            riff_chunk_id: Self::ID_SPECIFIER,
            riff_chunk_size: data.data_chunk_size + Self::CHUNK_MINIMUM_SIZE,
            file_format_type: Self::TYPE_SPECIFIER,
        }
    }

    /// `io::Read + io::Seek`から`Self`の情報を取得して作る。
    pub fn from_bufread<T>(reader: &mut T) -> Option<Self>
    where
        T: io::Read + io::Seek,
    {
        let mut buffer = [0u8; Self::STRUCTURE_SIZE];
        reader.read(&mut buffer[..]).expect("Failed to read riff header.");

        // ヘッダーがちゃんとしているかを確認してから返す。失敗したらそのまま終了。
        let maybe_header: Self = unsafe { std::ptr::read(buffer.as_ptr() as *const _) };

        // riff_idの確認。
        {
            let riff_id = std::str::from_utf8(&maybe_header.riff_chunk_id).unwrap();
            assert!(riff_id == "RIFF");
        }
        // format_type_idの確認。
        {
            let type_id = std::str::from_utf8(&maybe_header.file_format_type).unwrap();
            assert!(type_id == "WAVE");
        }

        Some(maybe_header)
    }

    /// [`WaveRiffHeader`]の情報を[`std::io::Write`]ストリームに書き込む。
    /// `writer`は[`std::io::Write`]と[`std::io::Seek`]を実装していること。
    pub fn write<T>(&self, writer: &mut T)
    where
        T: io::Write + io::Seek,
    {
        let mut buffer = [0u8; Self::STRUCTURE_SIZE];
        unsafe {
            let cloned = (*self).clone();
            std::ptr::write(buffer.as_mut_ptr() as *mut _, cloned);
        }
        writer.write(&buffer).expect("Failed to write WaveRiffHeader to writer.");
    }

    /// ヘッダーからデータバッファーのサイズを返す。
    pub fn data_chunk_size(&self) -> usize {
        assert!(self.riff_chunk_size >= Self::CHUNK_MINIMUM_SIZE);
        (self.riff_chunk_size - Self::CHUNK_MINIMUM_SIZE) as usize
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
struct WaveFmtHeader {
    /// `"fmt "`と同様
    fmt_chunk_id: [u8; 4],
    /// [`WaveFmtHeader::CHUNK_SIZE`]と同様
    fmt_chunk_size: u32,
    wave_format_type: u16,
    pub channel: u16,
    samples_per_sec: u32,
    ///
    bytes_per_sec: u32,
    /// チャンネルを含む各サンプルの総サイズ。
    /// もしチャンネルを分離した本当の各サンプルのサイズが取得したい場合には
    /// [`WaveFmtHeader::unit_block_size`]メソッドを使う。
    block_size: u16,
    bits_per_sample: u16,
}

const_assert_eq!(WaveFmtHeader::STRUCTURE_SIZE, 24usize);

impl WaveFmtHeader {
    const STRUCTURE_SIZE: usize = std::mem::size_of::<WaveFmtHeader>();
    const CHUNK_SIZE: u32 = 16;
    const ID_SPECIFIER: [u8; 4] = ['f' as u8, 'm' as u8, 't' as u8, ' ' as u8];

    /// [`WaveSound`]から[`WaveFmtHeader`]を生成する。
    pub fn from_wave_sound(sound: &WaveSound) -> Self {
        let channel = 1;
        let unit_block_size = sound.format.bits_per_sample.to_byte_size();
        let block_size = (unit_block_size as u16) * channel;

        Self {
            fmt_chunk_id: Self::ID_SPECIFIER,
            fmt_chunk_size: Self::CHUNK_SIZE,
            wave_format_type: 1,
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
            assert!(maybe_header.fmt_chunk_size == 16);
        }

        Some(maybe_header)
    }

    /// [`WaveFmtHeader`]の情報を[`std::io::Write`]ストリームに書き込む。
    /// `writer`は[`std::io::Write`]と[`std::io::Seek`]を実装していること。
    pub fn write<T>(&self, writer: &mut T)
    where
        T: io::Write + io::Seek,
    {
        let mut buffer = [0u8; Self::STRUCTURE_SIZE];
        unsafe {
            std::ptr::write(buffer.as_mut_ptr() as *mut _, (*self).clone());
        }
        writer.write(&buffer).expect("Failed to write WaveFmtHeader to writer.");
    }

    /// １個のチャンネルのブロックサイズを返す。
    pub fn unit_block_size(&self) -> usize {
        let block_size = self.block_size as usize;
        block_size / (self.channel as usize)
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
struct WaveFactChunk {
    fact_chunk_id: [u8; 4],
    fact_chunk_size: u32,
    sample_length: u32,
}

const_assert_eq!(WaveFactChunk::STRUCTURE_SIZE, 12usize);

impl WaveFactChunk {
    const STRUCTURE_SIZE: usize = std::mem::size_of::<WaveFactChunk>();

    /// `io::Read + io::Seek`から`Self`が読み取れるかを確認する。
    pub fn can_be_chunk<T>(reader: &mut T) -> bool
    where
        T: io::Read + io::Seek,
    {
        let mut id_buffer = [0u8; 4];
        let read_size = reader.read(&mut id_buffer[..]).expect("Failed to read fact header.");

        let id = std::str::from_utf8(&id_buffer).unwrap();
        let result = id == "fact";

        let return_size = (read_size as i64) * -1i64;
        reader.seek(io::SeekFrom::Current(return_size)).expect("Failed to see");

        result
    }

    /// `io::Read + io::Seek`から`Self`の情報を取得して作る。
    pub fn from_bufread<T>(reader: &mut T) -> Option<Self>
    where
        T: io::Read + io::Seek,
    {
        let mut buffer = [0u8; Self::STRUCTURE_SIZE];
        reader.read(&mut buffer[..]).expect("Failed to read fact header.");

        // ヘッダーがちゃんとしているかを確認してから返す。失敗したらそのまま終了。
        let maybe_header: Self = unsafe { std::ptr::read(buffer.as_ptr() as *const _) };

        // fact_idの確認。
        {
            let id = std::str::from_utf8(&maybe_header.fact_chunk_id).unwrap();
            assert!(id == "fact");
        }
        // fact_chunk_sizeの確認。
        {
            assert!(maybe_header.fact_chunk_size == 4);
        }

        Some(maybe_header)
    }

    /// [`WaveFactChunk`]の情報を[`std::io::Write`]ストリームに書き込む。
    /// `reader`は[`std::io::Write`]と[`std::io::Seek`]を実装していること。
    pub fn write<T>(&self, writer: &mut T)
    where
        T: io::Write + io::Seek,
    {
        let mut buffer = [0u8; Self::STRUCTURE_SIZE];
        unsafe {
            std::ptr::write(buffer.as_mut_ptr() as *mut _, (*self).clone());
        }
        writer.write(&buffer).expect("Failed to write WaveFactChunk to writer.");
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
struct WaveDataChunk {
    data_chunk_id: [u8; 4],
    pub data_chunk_size: u32,
}

const_assert_eq!(WaveDataChunk::STRUCTURE_SIZE, 8usize);

impl WaveDataChunk {
    const STRUCTURE_SIZE: usize = std::mem::size_of::<WaveDataChunk>();
    const ID_SPECIFIER: [u8; 4] = ['d' as u8, 'a' as u8, 't' as u8, 'a' as u8];

    /// [`WaveSound`]から[`WaveDataChunk`]を生成する。
    pub fn from_wave_sound(sound: &WaveSound) -> Self {
        let uniformed_unit_samples_count = sound.completed_samples_count() as u32;
        let bytes_of_converted = sound.format.bits_per_sample.to_u32() / 8;
        let data_chunk_size = uniformed_unit_samples_count * bytes_of_converted;

        Self {
            data_chunk_id: Self::ID_SPECIFIER,
            data_chunk_size,
        }
    }

    /// `io::Read + io::Seek`から`Self`の情報を取得して作る。
    pub fn from_bufread<T>(reader: &mut T) -> Option<Self>
    where
        T: io::Read + io::Seek,
    {
        let mut buffer = [0u8; Self::STRUCTURE_SIZE];
        reader.read(&mut buffer[..]).expect("Failed to read fact header.");

        // ヘッダーがちゃんとしているかを確認してから返す。失敗したらそのまま終了。
        let maybe_header: Self = unsafe { std::ptr::read(buffer.as_ptr() as *const _) };

        // chunk_idの確認。
        {
            let id = std::str::from_utf8(&maybe_header.data_chunk_id).unwrap();
            assert!(id == "data");
        }

        Some(maybe_header)
    }

    /// [`WaveDataChunk`]の情報を[`std::io::Write`]ストリームに書き込む。
    /// `writer`は[`std::io::Write`]と[`std::io::Seek`]を実装していること。
    pub fn write<T>(&self, writer: &mut T)
    where
        T: io::Write + io::Seek,
    {
        let mut buffer = [0u8; Self::STRUCTURE_SIZE];
        unsafe {
            std::ptr::write(buffer.as_mut_ptr() as *mut _, (*self).clone());
        }
        writer.write(&buffer).expect("Failed to write WaveDataChunk to writer.");
    }
}

/// 音源の情報を保持するコンテナ。
/// wavファイルからの読み込みやwavファイルへの書き込み、その他簡単なフィルタリング機能ができる。
#[derive(Debug)]
pub struct WaveContainer {
    riff: WaveRiffHeader,
    fmt: WaveFmtHeader,
    /// 非PCM形式のwaveの場合、`WaveFactChunk`が存在する。
    fact: Option<WaveFactChunk>,
    data: WaveDataChunk,
    /// 音源のバッファを平準化して保持する。
    uniformed_buffer: Vec<UniformedSample>,
}

impl WaveContainer {
    ///
    pub fn from_bufread<T>(reader: &mut T) -> Option<Self>
    where
        T: io::Read + io::Seek,
    {
        const MIMINUM_SIZE: usize = mem::size_of::<WaveRiffHeader>()
            + mem::size_of::<WaveFmtHeader>()
            + mem::size_of::<WaveFactChunk>()
            + mem::size_of::<WaveDataChunk>();

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
        let wave_riff_header = WaveRiffHeader::from_bufread(reader).expect("Failed to get riff header.");
        let wave_fmt_header = WaveFmtHeader::from_bufread(reader).expect("Failed to get fmt header.");
        let wave_fact_chunk = {
            if WaveFactChunk::can_be_chunk(reader) {
                Some(WaveFactChunk::from_bufread(reader).expect("Failed to get fact chunk."))
            } else {
                None
            }
        };
        let wave_data_chunk = WaveDataChunk::from_bufread(reader).expect("Failed to get data chunk");
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
        let data = WaveDataChunk::from_wave_sound(sound);
        let riff = WaveRiffHeader::from_data_chunk(&data);
        let fmt = WaveFmtHeader::from_wave_sound(sound);

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
    pub(crate) fn from_uniformed_sample_buffer(original: &Self, uniformed_buffer: Vec<UniformedSample>) -> Self {
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

        // そしてバッファーから量子化ビットとブロックサイズに合わせて別リストに変換し書き込ませる。
        let converted_buffer: Vec<i16> = {
            // `unit_block_size`は各ユニットブロックのメモリ空間を、
            // `bits_per_sample`は`UniformSample`からどのように数値に変換するかを表す。
            let unit_block_size = self.unit_block_size();
            let bits_per_sample = self.bits_per_sample();
            assert_eq!(bits_per_sample, 16);
            assert_eq!(unit_block_size, 2);

            self.uniformed_buffer.iter().map(|v| v.to_16bits()).collect()
        };

        let converted_buffer_slice = unsafe {
            let p_buffer = converted_buffer.as_ptr() as *const u8;
            std::slice::from_raw_parts(p_buffer, converted_buffer.len() * 2)
        };

        writer
            .write(&converted_buffer_slice)
            .expect("Failed to write Buffer to writer.");
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
    pub fn sound_length(&self) -> f32 {
        let items_per_sec = (self.fmt.samples_per_sec as usize) * (self.fmt.channel as usize);
        let sound_length = (self.uniformed_buffer.len() as f64) / (items_per_sec as f64);
        sound_length as f32
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
        // 今はチャンネルをMONOに限定する。
        assert!(self.fmt.channel == 1);
        if time >= (self.sound_length() as f64) {
            return None;
        }

        let index = ((self.fmt.samples_per_sec as f64) * time).floor() as usize;
        Some(self.uniformed_buffer[index])
    }

    /// サンプルが入っているバッファーのSliceを貸す形で返す。
    pub(crate) fn uniformed_sample_buffer(&self) -> &'_ [UniformedSample] {
        &self.uniformed_buffer
    }
}
