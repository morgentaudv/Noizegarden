use std::{fs, io};

use soundprog::wave::{
    analyze::{ETransformMethod, FrequencyAnalyzer, FrequencyTransformer},
    container::WaveContainer,
};

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
        frequency_start: 1.0,
        frequency_length: 22000.0,
        sample_counts: 8000,
        ..Default::default()
    };

    let frequencies = dft_analyzer.analyze_frequencies(&wave_container).unwrap();
    for frequency in &frequencies {
        println!("{:?}", frequency);
    }

    let uniformed_sample_buffer = FrequencyTransformer {
        transform_method: ETransformMethod::IDFT,
    }
    .transform_frequencies(&wave_container, &frequencies)
    .unwrap();

    // IDFTで音がちゃんと合成できるかを確認する。
    //dft_analyzer.create_sample_buffer(&wave_container, &frequencies);
}
