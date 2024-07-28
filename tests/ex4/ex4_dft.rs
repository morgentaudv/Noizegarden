use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::analyze::analyzer::FrequencyAnalyzer;
use soundprog::wave::analyze::transformer::FrequencyTransformer;
use soundprog::wave::{
    analyze::method::ETransformMethod,
    container::{WaveBuilder, WaveContainer},
};

#[test]
fn test_dft() {
    const READ_FILE_PATH: &'static str = "assets/ex4/sine_500hz.wav";
    const WRITE_FILE_PATH: &'static str = "assets/ex4/sine_500hz_idft_240728.wav";

    // DFTを使って分析。
    let wave_container = {
        let source_file = fs::File::open(READ_FILE_PATH).expect("Could not find file.");
        let mut reader = io::BufReader::new(source_file);

        WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
    };
    let dft_analyzer = FrequencyAnalyzer {
        start_sample_index: 0,
        frequency_start: 0.0,
        sample_rate: wave_container.samples_per_second(),
        samples_count: 8000,
        ..Default::default()
    };
    let frequencies = dft_analyzer.analyze_container(&wave_container).unwrap();

    // IDFTで音がちゃんと合成できるかを確認する。
    let uniformed_samples = FrequencyTransformer {
        transform_method: ETransformMethod::IDFT,
    }
    .transform_frequencies(&frequencies)
    .unwrap();

    let new_wave_container = WaveBuilder {
        samples_per_sec: wave_container.samples_per_second(),
        bits_per_sample: wave_container.bits_per_sample() as u16,
    }
    .build_container(uniformed_samples)
    .unwrap();
    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect("Could not create file.");
        let mut writer = io::BufWriter::new(dest_file);
        new_wave_container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
