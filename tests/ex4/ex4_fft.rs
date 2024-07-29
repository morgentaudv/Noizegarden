use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::analyze::{
    analyzer::{FrequencyAnalyzer, FrequencyAnalyzerV2, WaveContainerSetting},
    method::EAnalyzeMethod,
    transformer::EExportSampleCountMode,
};
use soundprog::wave::analyze::{transformer::FrequencyTransformer, window::EWindowFunction};
use soundprog::wave::{
    analyze::method::ETransformMethod,
    container::{WaveBuilder, WaveContainer},
};

#[test]
fn test_fft() {
    const READ_FILE_PATH: &'static str = "assets/ex4/sine_500hz.wav";
    const WRITE_FILE_PATH: &'static str = "assets/ex4/sine_500hz_ifft_240729.wav";

    // DFTを使って分析。
    let wave_container = {
        let source_file = fs::File::open(READ_FILE_PATH).expect("Could not find file.");
        let mut reader = io::BufReader::new(source_file);

        WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
    };
    let samples_count = wave_container.uniformed_sample_buffer().len();
    let frequencies = {
        let analyzer = FrequencyAnalyzerV2 {
            analyze_method: EAnalyzeMethod::FFT,
            frequency_start: 0.0,
            frequency_width: 44100.0,
            frequency_bin_count: 16384,
            window_function: EWindowFunction::None,
        };

        let setting = WaveContainerSetting {
            container: &wave_container,
            start_sample_index: 0,
            samples_count: 16384,
        };
        analyzer.analyze_container(&setting).unwrap()
    };

    // IFFTで音がちゃんと合成できるかを確認する。
    let uniformed_samples = FrequencyTransformer {
        transform_method: ETransformMethod::IFFT,
        sample_count_mode: EExportSampleCountMode::Fixed(samples_count),
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
