use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    container::WaveContainer,
    filter::{EEdgeFrequency, EFilter},
};

#[test]
fn ex7_1() {
    const READ_FILE_PATH: &'static str = "assets/ex7/pulse_train.wav";
    const WRITE_FILE_PATH: &'static str = "assets/ex7/ex7_1.wav";

    let wave_container = {
        let source_file = fs::File::open(READ_FILE_PATH).expect(&format!("Could not find {}.", READ_FILE_PATH));
        let mut reader = io::BufReader::new(source_file);
        WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
    };
    let new_container = EFilter::IIRLowPass {
        edge_frequency: EEdgeFrequency::ChangeBySample(|sample_i, sample_count, _| {
            const BASE_EDGE_FREQUENCY: f64 = 10000.0;
            let sample_rate = (sample_i as f64) / (sample_count as f64);

            BASE_EDGE_FREQUENCY * (-5.0 * sample_rate).exp()
        }),
        quality_factor: 2f64.sqrt().recip(),
        adsr: None,
    }
    .apply_to_wave_container(&wave_container);

    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect(&format!("Could not create {}.", WRITE_FILE_PATH));
        let mut writer = io::BufWriter::new(dest_file);
        new_container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
