use std::io;

#[repr(C)]
#[derive(Debug, Clone)]
pub(crate) struct LowWaveDataChunk {
    data_chunk_id: [u8; 4],
    pub data_chunk_size: u32,
}

const_assert_eq!(LowWaveDataChunk::STRUCTURE_SIZE, 8usize);

impl LowWaveDataChunk {
    const STRUCTURE_SIZE: usize = std::mem::size_of::<LowWaveDataChunk>();
    const ID_SPECIFIER: [u8; 4] = ['d' as u8, 'a' as u8, 't' as u8, 'a' as u8];

    pub fn from_chunk_size(data_chunk_size: u32) -> Self {
        Self {
            data_chunk_id: LowWaveDataChunk::ID_SPECIFIER,
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

    /// [`LowWaveDataChunk`]の情報を[`std::io::Write`]ストリームに書き込む。
    /// `writer`は[`std::io::Write`]と[`std::io::Seek`]を実装していること。
    pub fn write<T>(&self, writer: &mut T)
    where
        T: io::Write + io::Seek,
    {
        let mut buffer = [0u8; Self::STRUCTURE_SIZE];
        unsafe {
            std::ptr::write(buffer.as_mut_ptr() as *mut _, (*self).clone());
        }
        writer.write(&buffer).expect("Failed to write LowWaveDataChunk to writer.");
    }
}
