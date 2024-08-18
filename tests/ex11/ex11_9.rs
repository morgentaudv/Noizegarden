use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::container::{wav::adpcm::IMAADPCMWriter, WaveContainer};

#[test]
fn ex11_9() {
    const READ_FILE_PATH: &'static str = "assets/ex7/vocal.wav";
    //const READ_FILE_PATH: &'static str = "assets/ex11/sine_2s.wav";
    const WRITE_FILE_PATH: &'static str = "assets/ex11/ex11_9_to_adpcm.wav";

    let wave_container = {
        let source_file = fs::File::open(READ_FILE_PATH).expect(&format!("Could not find {}.", READ_FILE_PATH));
        let mut reader = io::BufReader::new(source_file);

        WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
    };

    // LPCMからIMA-ADPCMに変換。
    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect(&format!("Could not create {}.", WRITE_FILE_PATH));
        let mut writer = io::BufWriter::new(dest_file);

        IMAADPCMWriter {
            source_container: &wave_container,
        }
        .write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
