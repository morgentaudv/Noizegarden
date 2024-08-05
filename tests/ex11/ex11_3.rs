use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    analyze::window::EWindowFunction,
    container::{WaveBuilder, WaveContainer},
    stretch::{
        pitch::{self, PitchShifterBufferSetting, PitchShifterBuilder},
        time::{TimeStretcherBufferSetting, TimeStretcherBuilder},
    },
};

#[test]
fn ex11_3() {
    //const READ_FILE_PATH: &'static str = "assets/ex11/sine_2s.wav";
    const READ_FILE_PATH: &'static str = "assets/ex6/drum.wav";
    const WRITE_FILE_PATH: &'static str = "assets/ex11/ex11_3_output.wav";

    let wave_container = {
        let source_file = fs::File::open(READ_FILE_PATH).expect(&format!("Could not find {}.", READ_FILE_PATH));
        let mut reader = io::BufReader::new(source_file);

        WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
    };

    let pitch_rate = 0.5;
    let stretch_rate = 1.0 / pitch_rate;

    // Pitchを上げて
    let pitch_buffer = {
        let setting = PitchShifterBufferSetting {
            buffer: wave_container.uniformed_sample_buffer(),
        };

        PitchShifterBuilder::default()
            .pitch_rate(pitch_rate)
            .window_size(128)
            .window_function(EWindowFunction::None)
            .build()
            .unwrap()
            .process_with_buffer(&setting)
            .unwrap()
    };

    // 尺を戻す
    let time_buffer = {
        let setting = TimeStretcherBufferSetting { buffer: &pitch_buffer };
        let original_fs = wave_container.samples_per_second();
        let template_size = (original_fs as f64 * 0.01) as usize;
        let p_min = (original_fs as f64 * 0.005) as usize;
        let p_max = (original_fs as f64 * 0.02) as usize;

        TimeStretcherBuilder::default()
            .template_size(template_size)
            .shrink_rate(stretch_rate)
            .sample_period_min(p_min)
            .sample_period_length(p_max - p_min)
            .build()
            .unwrap()
            .process_with_buffer(&setting)
            .unwrap()
    };

    let new_wave_container = WaveBuilder {
        samples_per_sec: wave_container.samples_per_second(),
        bits_per_sample: wave_container.bits_per_sample() as u16,
    }
    .build_container(time_buffer)
    .unwrap();

    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect(&format!("Could not create {}.", WRITE_FILE_PATH));
        let mut writer = io::BufWriter::new(dest_file);
        new_wave_container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
