use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    analyze::{EAnalyzeMethod, ETransformMethod, FrequencyAnalyzerBuilder, FrequencyTransformer},
    container::{WaveBuilder, WaveContainer},
};

#[test]
fn test_dft_hann() {
    const FILE_PATH: &'static str = "assets/ex4/sine_500hz.wav";
    const WRITE_FILE_PATH: &'static str = "assets/ex4/sine_500hz_ifft.wav";

    let wave_container = {
        let source_file = fs::File::open(FILE_PATH).expect("Could not find a.wav.");
        let mut reader = io::BufReader::new(source_file);

        WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
    };

    // FFT
    let frequencies = {
        let analyzer = FrequencyAnalyzerBuilder::default()
            .start_sample_index(0)
            .frequency_start(0.0)
            .samples_count(4096)
            .window_function(None)
            .analyze_method(EAnalyzeMethod::FFT)
            .build()
            .unwrap();

        analyzer.analyze_container(&wave_container).unwrap()
    };

    // IDFTで音がちゃんと合成できるかを確認する。
    let uniformed_samples = FrequencyTransformer {
        transform_method: ETransformMethod::IFFT,
    }
    .transform_frequencies(&wave_container, &frequencies)
    .unwrap();

    let new_wave_container = WaveBuilder {
        samples_per_sec: 8000,
        bits_per_sample: 16,
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
