use std::{io, mem};
use std::io::{SeekFrom};
use crate::wave::container::wav::try_read_wave_header_id_str;

#[repr(C)]
#[derive(Debug, Clone)]
pub(crate) struct LowWaveQualityHeader {
    /// "qlty"と同じ
    chunk_id: [u8; 4],
    chunk_size: u32,
    /// chunk_size分のサイズを持つ。
    chunk_data: Vec<u8>,
}

impl LowWaveQualityHeader {
    pub fn from_bufread<T>(reader: &mut T) -> Option<Self>
    where T: io::Read + io::Seek
    {
        let id = try_read_wave_header_id_str(reader);
        if id != "qlty" {
            return None;
        }
        reader.seek(SeekFrom::Current(4)).unwrap();

        let chunk_size: u32 = {
            const_assert_eq!(size_of::<u32>(), 4usize);

            let mut chunk_size_buffer = [0u8; 4];
            reader.read(&mut chunk_size_buffer).unwrap();

            unsafe { mem::transmute(chunk_size_buffer) }
        };

        let mut chunk_buffer = vec![0u8; chunk_size as usize];
        reader.read(&mut chunk_buffer).unwrap();

        Some(Self {
            chunk_id: "qlty".as_bytes().try_into().unwrap(),
            chunk_size,
            chunk_data: vec![],
        })
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
