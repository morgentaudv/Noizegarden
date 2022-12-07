use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    container::WaveContainer,
    filter::{EEdgeFrequency, EFilter, ESourceFilter},
    sample::UniformedSample,
};

#[test]
fn ex7_3() {
    const READ_FILE_PATH: &'static str = "assets/ex7/pulse_train2.wav";
    const WRITE_FILE_PATH: &'static str = "assets/ex7/ex7_3_o.wav";

    let wave_container = {
        let source_file = fs::File::open(READ_FILE_PATH).expect(&format!("Could not find {}.", READ_FILE_PATH));
        let mut reader = io::BufReader::new(source_file);
        WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
    };

    // フィルタリングする
    let filtered_buffer = {
        let sample_buffer_length = wave_container.uniformed_sample_buffer().len();
        let mut buffer = vec![UniformedSample::default(); sample_buffer_length];

        let target_frequencies = [500.0, 800.0, 2500.0, 3500.0];
        for frequency in target_frequencies {
            let filtered_buffer = EFilter::IIRBandPass {
                center_frequency: EEdgeFrequency::Constant(frequency),
                quality_factor: frequency / 100.0,
            }
            .apply_to_wave_container(&wave_container)
            .uniformed_sample_buffer()
            .to_vec();

            for write_i in 0..filtered_buffer.len() {
                buffer[write_i] += filtered_buffer[write_i];
            }
        }

        buffer
    };

    // ディエンファシスする。
    let deemphasised_buffer = ESourceFilter::Deemphasizer { coefficient: 0.98 }.apply_to_buffer(&filtered_buffer);

    {
        let new_container = WaveContainer::from_uniformed_sample_buffer(&wave_container, deemphasised_buffer);

        let dest_file = fs::File::create(WRITE_FILE_PATH).expect(&format!("Could not create {}.", WRITE_FILE_PATH));
        let mut writer = io::BufWriter::new(dest_file);
        new_container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
