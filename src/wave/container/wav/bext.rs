use std::{io, mem};
use std::io::{SeekFrom};
use crate::wave::container::wav::try_read_wave_header_id_str;

#[repr(C)]
#[derive(Clone, Debug)]
pub(crate) struct LowWaveBextHeader {
    /// "bext"と同じ
    chunk_id: [u8; 4],
    chunk_size: u32,
    /// chunk_size分のサイズを持つ。
    chunk_data: Vec<u8>,
}

impl LowWaveBextHeader {
    /// `io::Read + io::Seek`から`Self`の情報を取得して作る。
    pub fn from_bufread<T>(reader: &mut T) -> Option<Self>
    where T: io::Read + io::Seek
    {
        let id = try_read_wave_header_id_str(reader);
        if id != "bext" {
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
            chunk_id: "bext".as_bytes().try_into().unwrap(),
            chunk_size,
            chunk_data: vec![],
        })
    }

    /// [`LowWaveBextHeader`]の情報を[`io::Write`]ストリームに書き込む。
    pub fn write<T>(&self, writer: &mut T)
    where
        T: io::Write + io::Seek,
    {
        let data_size = self.chunk_data.len();
        let total_size = size_of_val(&self.chunk_id) + size_of_val(&self.chunk_size) + data_size;

        let mut buffer = vec![0u8; total_size];
        unsafe {
            let mut pointer = buffer.as_mut_ptr();
            std::ptr::write(pointer as *mut _, self.chunk_id);
            pointer = pointer.add(size_of_val(&self.chunk_id));

            std::ptr::write(pointer as *mut _, self.chunk_size);
            pointer = pointer.add(size_of_val(&self.chunk_size));

            std::ptr::write(pointer as *mut _, self.chunk_data.as_ptr());
        }

        writer.write_all(&buffer).expect("Failed to write Bext chunk to writer.");
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
