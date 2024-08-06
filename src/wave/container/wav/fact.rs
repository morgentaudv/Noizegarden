use std::io;

#[repr(C)]
#[derive(Debug, Clone)]
pub(crate) struct LowWaveFactChunk {
    fact_chunk_id: [u8; 4],
    fact_chunk_size: u32,
    sample_length: u32,
}

const_assert_eq!(LowWaveFactChunk::STRUCTURE_SIZE, 12usize);

impl LowWaveFactChunk {
    const STRUCTURE_SIZE: usize = std::mem::size_of::<LowWaveFactChunk>();

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

    /// [`LowWaveFactChunk`]の情報を[`std::io::Write`]ストリームに書き込む。
    /// `reader`は[`std::io::Write`]と[`std::io::Seek`]を実装していること。
    pub fn write<T>(&self, writer: &mut T)
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
