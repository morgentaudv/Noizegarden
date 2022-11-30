use std::{fs, io};

use soundprog::wave::{analyze::FrequencyAnalyzer, container::WaveContainer};

#[test]
fn test_dft() {
    const FILE_PATH: &'static str = "assets/ex4/sine_500hz.wav";

    let wave_container = {
        let source_file = fs::File::open(FILE_PATH).expect("Could not find a.wav.");
        let mut reader = io::BufReader::new(source_file);

        WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
    };
    let dft_analyzer = FrequencyAnalyzer {
        time_start: 0.0,
        time_length: 1.0,
        time_precision: (1.0 / 64f64),
        frequency_start: 0f32,
        frequency_length: 64f32,
        frequency_precision: 1f32,
        ..Default::default()
    };

    let frequencies = dft_analyzer.analyze_frequencies(&wave_container).unwrap();
}
