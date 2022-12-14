use std::{
    fs,
    io::{self, Write},
};

use itertools::Itertools;
use soundprog::wave::{
    analyze::{EAnalyzeMethod, ETransformMethod, FrequencyAnalyzer, FrequencyTransformer, SineFrequency},
    container::WaveContainer,
    filter::ESourceFilter,
    sample::UniformedSample,
};

fn create_wave_container_from_file(file_path: &str) -> WaveContainer {
    let source_file = fs::File::open(file_path).expect(&format!("Could not find {}.", file_path));
    let mut reader = io::BufReader::new(source_file);
    WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
}

#[test]
fn ex7_4() {
    const VOCAL_FILE_PATH: &'static str = "assets/ex7/vocal.wav";
    const SYNTH_FILE_PATH: &'static str = "assets/ex7/synth.wav";
    const WRITE_FILE_PATH: &'static str = "assets/ex7/ex7_4_vocoder.wav";

    let synth_container = create_wave_container_from_file(SYNTH_FILE_PATH);
    let vocal_container = {
        let original_container = create_wave_container_from_file(VOCAL_FILE_PATH);

        // まず音声の高周波数は低幅という特性を持っているのでプリエンファシスしなければならない。
        let filtered_buffer = ESourceFilter::PreEmphasizer { coefficient: 0.98 }
            .apply_to_buffer(original_container.uniformed_sample_buffer());
        WaveContainer::from_uniformed_sample_buffer(&original_container, filtered_buffer)
    };

    assert!(vocal_container.channel() == 1);
    let transform_compute_count = 1024usize;
    let half_compute_count = transform_compute_count >> 1;
    let vocal_container_sample_len = vocal_container.uniformed_sample_buffer().len();
    let band_width = 8usize;

    // Use half overlapping.
    let compute_frame_count = (vocal_container_sample_len / half_compute_count) - 1;

    // FFTで使うAnalzyer情報を記す。
    let freq_analyzer = FrequencyAnalyzer {
        start_sample_index: 0,
        frequency_start: 1.0,
        samples_count: transform_compute_count,
        window_function: None,
        analyze_method: EAnalyzeMethod::FFT,
    };
    // IFFTに使うTransformerを記す。
    let transformer = FrequencyTransformer {
        transform_method: ETransformMethod::IFFT,
    };

    let mut new_buffer = vec![];
    new_buffer.resize(vocal_container_sample_len, UniformedSample::default());
    for frame_i in 0..compute_frame_count {
        let begin_sample_i = frame_i * half_compute_count;
        let end_sample_i = (begin_sample_i + transform_compute_count).min(vocal_container_sample_len);
        let sample_length = end_sample_i - begin_sample_i;

        // X(n)
        let target_synth_freqs = {
            let buffer = synth_container
                .uniformed_sample_buffer()
                .iter()
                .skip(begin_sample_i)
                .take(sample_length)
                .map(|v| *v)
                .collect_vec();

            freq_analyzer
                .analyze_sample_buffer(&buffer)
                .expect("Failed to analyze input signal buffer.")
        };

        // B(k)
        let target_vocal_freqs = {
            let buffer = vocal_container
                .uniformed_sample_buffer()
                .iter()
                .skip(begin_sample_i)
                .take(sample_length)
                .map(|v| *v)
                .collect_vec();

            // Phaseは全部捨てる。
            let mut frequencies = freq_analyzer
                .analyze_sample_buffer(&buffer)
                .expect("Failed to analyze filter signal buffer.")
                .into_iter()
                .map(|v| SineFrequency {
                    frequency: v.frequency,
                    amplitude: v.amplitude,
                    phase: 0.0,
                })
                .collect_vec();

            // B(k)に周波数エンベロープをかけて平坦化する。これでロボット音にするフィルターの加工は完了
            for band_i in (0..half_compute_count).step_by(band_width) {
                let summed_amplitude: f64 = frequencies.iter().skip(band_i).take(band_width).map(|v| v.amplitude).sum();

                let normalized_amplitude = summed_amplitude / (band_width as f64);
                for vocal_freq in frequencies.iter_mut().skip(band_i).take(band_width) {
                    vocal_freq.amplitude = normalized_amplitude;
                }
            }

            // 周期になるために`half_compute_count`分を繰り返す。
            assert!(half_compute_count == (transform_compute_count >> 1));
            frequencies[0].amplitude = 0.0;
            for copy_i in 0..half_compute_count {
                let write_i = half_compute_count + copy_i;
                frequencies[write_i] = frequencies[copy_i];
            }

            frequencies
        };

        let processed_buffer = target_synth_freqs
            .iter()
            .zip(&target_vocal_freqs)
            .map(|(synth, vocal)| {
                SineFrequency::from_complex_f64(synth.frequency, synth.to_complex_f64() * vocal.to_complex_f64())
            })
            .collect_vec();
        let frame_result_buffer = transformer.transform_frequencies(&processed_buffer).expect("msg");

        // 適切な位置に書き込む。
        for write_i in begin_sample_i..end_sample_i {
            let load_i = write_i - begin_sample_i;
            new_buffer[write_i] = frame_result_buffer[load_i];
        }
    }

    {
        let new_container = WaveContainer::from_uniformed_sample_buffer(&vocal_container, new_buffer);

        let dest_file = fs::File::create(WRITE_FILE_PATH).expect(&format!("Could not create {}.", WRITE_FILE_PATH));
        let mut writer = io::BufWriter::new(dest_file);
        new_container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
