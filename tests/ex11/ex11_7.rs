use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::container::{WaveBuilder, WaveContainer};

#[test]
fn ex11_7() {
    const READ_FILE_PATH: &'static str = "assets/ex7/vocal.wav";
    //const READ_FILE_PATH: &'static str = "assets/ex11/sine_2s.wav";
    const WRITE_FILE_PATH: &'static str = "assets/ex11/ex11_7_ulaw.wav";

    let wave_container = {
        let source_file = fs::File::open(READ_FILE_PATH).expect(&format!("Could not find {}.", READ_FILE_PATH));
        let mut reader = io::BufReader::new(source_file);

        WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
    };

    // LPCMからPCMU(u-law)に変換
    let new_wave_container = WaveBuilder::from_container_to_ulaw(&wave_container).unwrap();

    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect(&format!("Could not create {}.", WRITE_FILE_PATH));
        let mut writer = io::BufWriter::new(dest_file);
        new_wave_container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
