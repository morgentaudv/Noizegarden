use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::container::WaveContainer;

#[test]
fn read_and_write_file() {
    const FILE_PATH: &'static str = "assets/ex1/a.wav";
    const WRITE_FILE_PATH: &'static str = "assets/ex1/b.wav";

    let wave_container = {
        let source_file = fs::File::open(FILE_PATH).expect("Could not find a.wav.");
        let mut reader = io::BufReader::new(source_file);

        WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
    };

    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect("Could not create b.wav.");
        let mut writer = io::BufWriter::new(dest_file);
        wave_container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
