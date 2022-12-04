use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{container::WaveContainer, sample::UniformedSample};

#[test]
fn ex6_5() {
    const READ_SOUND_FILE_PATH: &'static str = "assets/ex6/drum.wav";
    const READ_RESPONSE_FILE_PATH: &'static str = "assets/ex6/response.wav";
    const WRITE_FILE_PATH: &'static str = "assets/ex6/drum_reverved_ex6_5.wav";

    let drum_wave_container = {
        let source_file =
            fs::File::open(READ_SOUND_FILE_PATH).expect(&format!("Could not find {}.", READ_SOUND_FILE_PATH));
        let mut reader = io::BufReader::new(source_file);
        WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
    };
    let response_wave_container = {
        let source_file =
            fs::File::open(READ_RESPONSE_FILE_PATH).expect(&format!("Could not find {}.", READ_RESPONSE_FILE_PATH));
        let mut reader = io::BufReader::new(source_file);
        WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
    };

    // ここでは内部処理を行わずにresponseのバッファからフィルタリングを行う。
    let output_buffer = {
        // x(n - m)
        let drum_buffer = drum_wave_container.uniformed_sample_buffer();
        // b(m)
        let response_buffer = response_wave_container.uniformed_sample_buffer();
        let response_sample_count = response_buffer.len();

        let mut output_buffer = vec![];
        output_buffer.resize(drum_buffer.len(), UniformedSample::default());
        for write_i in 0..drum_buffer.len() {
            for filter_i in 0..response_sample_count {
                if write_i < filter_i {
                    continue;
                }

                let drum_load_i = write_i - filter_i;
                output_buffer[write_i] += response_buffer[filter_i].to_f64() * drum_buffer[drum_load_i];
            }
        }

        output_buffer
    };

    {
        let output_container = WaveContainer::from_uniformed_sample_buffer(&drum_wave_container, output_buffer);

        let dest_file = fs::File::create(WRITE_FILE_PATH).expect(&format!("Could not create {}.", WRITE_FILE_PATH));
        let mut writer = io::BufWriter::new(dest_file);
        output_container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
