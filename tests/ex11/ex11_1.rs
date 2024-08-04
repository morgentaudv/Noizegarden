use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    container::{WaveBuilder, WaveContainer},
    stretch::time::{TimeStretcherBufferSetting, TimeStretcherBuilder},
};

#[test]
fn ex11_1() {
    //const READ_FILE_PATH: &'static str = "assets/ex11/sine_2s.wav";
    const READ_FILE_PATH: &'static str = "assets/ex7/vocal.wav";
    const WRITE_FILE_PATH: &'static str = "assets/ex11/ex11_1_output.wav";
    const STRETCH_RATE: f64 = 0.5;
    assert!(STRETCH_RATE >= 0.5);

    let wave_container = {
        let source_file = fs::File::open(READ_FILE_PATH).expect(&format!("Could not find {}.", READ_FILE_PATH));
        let mut reader = io::BufReader::new(source_file);

        WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
    };

    // まず音源から周期性を把握する。
    let shrink_rate = STRETCH_RATE;
    let original_fs = wave_container.samples_per_second();
    let template_size = (original_fs as f64 * 0.01) as usize;
    let p_min = (original_fs as f64 * 0.005) as usize;
    let p_max = (original_fs as f64 * 0.02) as usize;

    let setting = TimeStretcherBufferSetting {
        buffer: wave_container.uniformed_sample_buffer(),
    };

    let result_buffer = TimeStretcherBuilder::default()
        .template_size(template_size)
        .shrink_rate(shrink_rate)
        .sample_period_min(p_min)
        .sample_period_length(p_max - p_min)
        .build()
        .unwrap()
        .process_with_buffer(&setting)
        .unwrap();

    let new_wave_container = WaveBuilder {
        samples_per_sec: wave_container.samples_per_second(),
        bits_per_sample: wave_container.bits_per_sample() as u16,
    }
    .build_container(result_buffer)
    .unwrap();

    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect(&format!("Could not create {}.", WRITE_FILE_PATH));
        let mut writer = io::BufWriter::new(dest_file);
        new_wave_container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
