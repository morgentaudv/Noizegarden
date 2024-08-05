use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    analyze::window::EWindowFunction,
    container::{WaveBuilder, WaveContainer},
    stretch::pitch::{PitchShifterBufferSetting, PitchShifterBuilder},
};

#[test]
fn ex11_3() {
    //const READ_FILE_PATH: &'static str = "assets/ex11/sine_2s.wav";
    const READ_FILE_PATH: &'static str = "assets/ex6/drum.wav";
    //const READ_FILE_PATH: &'static str = "assets/ex7/vocal.wav";
    const WRITE_FILE_PATH: &'static str = "assets/ex11/ex11_3_output.wav";

    let wave_container = {
        let source_file = fs::File::open(READ_FILE_PATH).expect(&format!("Could not find {}.", READ_FILE_PATH));
        let mut reader = io::BufReader::new(source_file);

        WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
    };

    // まず音源から周期性を把握する。
    let setting = PitchShifterBufferSetting {
        buffer: wave_container.uniformed_sample_buffer(),
    };

    let dst_buffer = PitchShifterBuilder::default()
        .pitch_rate(0.87)
        .window_size(128)
        .window_function(EWindowFunction::None)
        .build()
        .unwrap()
        .process_with_buffer(&setting)
        .unwrap();

    let new_wave_container = WaveBuilder {
        samples_per_sec: wave_container.samples_per_second(),
        bits_per_sample: wave_container.bits_per_sample() as u16,
    }
    .build_container(dst_buffer)
    .unwrap();

    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect(&format!("Could not create {}.", WRITE_FILE_PATH));
        let mut writer = io::BufWriter::new(dest_file);
        new_wave_container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
