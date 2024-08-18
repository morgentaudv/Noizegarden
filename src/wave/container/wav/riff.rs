use std::io;

use super::data::LowWaveDataChunk;

#[repr(C)]
#[derive(Debug, Clone)]
pub(crate) struct LowWaveRiffHeader {
    riff_chunk_id: [u8; 4],
    riff_chunk_size: u32,
    file_format_type: [u8; 4],
}

const_assert_eq!(std::mem::size_of::<LowWaveRiffHeader>(), 12usize);

impl LowWaveRiffHeader {
    const STRUCTURE_SIZE: usize = std::mem::size_of::<LowWaveRiffHeader>();
    const CHUNK_MINIMUM_SIZE: u32 = 48;
    const CHUNK_IMA_ADPCM_ADD_SIZE: u32 = 52;
    const ID_SPECIFIER: [u8; 4] = ['R' as u8, 'I' as u8, 'F' as u8, 'F' as u8];
    const TYPE_SPECIFIER: [u8; 4] = ['W' as u8, 'A' as u8, 'V' as u8, 'E' as u8];

    /// [`LowWaveDataChunk`]の設定から[`LowWaveRiffHeader`]を生成する。
    pub fn from_data_chunk(data: &LowWaveDataChunk) -> Self {
        Self {
            riff_chunk_id: Self::ID_SPECIFIER,
            riff_chunk_size: data.data_chunk_size + Self::CHUNK_MINIMUM_SIZE,
            file_format_type: Self::TYPE_SPECIFIER,
        }
    }

    /// [`Self::from_data_chunk`]と同じだが、IMA-ADPCM用のRIFFヘッダーを作る。
    pub fn from_data_chunk_with_ima_adpcm(data: &LowWaveDataChunk) -> Self {
        Self {
            riff_chunk_id: Self::ID_SPECIFIER,
            riff_chunk_size: data.data_chunk_size + Self::CHUNK_IMA_ADPCM_ADD_SIZE,
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

    /// [`LowWaveRiffHeader`]の情報を[`std::io::Write`]ストリームに書き込む。
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
        writer.write(&buffer).expect("Failed to write LowWaveRiffHeader to writer.");
    }

    /// ヘッダーからデータバッファーのサイズを返す。
    pub fn data_chunk_size(&self) -> usize {
        assert!(self.riff_chunk_size >= Self::CHUNK_MINIMUM_SIZE);
        (self.riff_chunk_size - Self::CHUNK_MINIMUM_SIZE) as usize
    }
}
