use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{container::WaveContainer, filter::EFilter};

#[test]
fn ex6_2() {
    const READ_FILE_PATH: &'static str = "assets/ex6/sine_500hz_3500hz.wav";
    const WRITE_FILE_PATH: &'static str = "assets/ex6/sine_500hz_ex6_2.wav";

    let wave_container = {
        let source_file = fs::File::open(READ_FILE_PATH).expect(&format!("Could not find {}.", READ_FILE_PATH));
        let mut reader = io::BufReader::new(source_file);

        WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
    };
    let new_container = EFilter::IIRLowPass {
        edge_frequency: 1000.0,
        quality_factor: 2f64.sqrt().recip(),
    }
    .apply_to_wave_container(&wave_container);

    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect(&format!("Could not create {}.", WRITE_FILE_PATH));
        let mut writer = io::BufWriter::new(dest_file);
        new_container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
