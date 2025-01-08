use std::io;
use std::io::{Read, Seek};

pub mod adpcm;
pub mod data;
pub mod fact;
pub mod fmt;
pub mod riff;
pub mod bext;
pub mod junk;

/// WavファイルのカーソルからヘッダーのIDと見られる4文字を読み込みした後、元の位置に戻す。
pub fn try_read_wave_header_id_str<T>(reader: &mut T) -> String
where
    T: io::Read + io::Seek,
{
    let mut buffer = [0u8; 4];
    reader.read(&mut buffer).expect("Failed to read fact header");
    // u8 4文字戻す。
    reader.seek(io::SeekFrom::Current(-4)).unwrap();

    let id = String::from_utf8_lossy(&buffer).to_string();
    id
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
