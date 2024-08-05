use std::{
    f64::consts::PI,
    fs,
    io::{self, Write},
};

use soundprog::{
    math::sinc,
    wave::{
        analyze::window::EWindowFunction,
        container::{WaveBuilder, WaveContainer},
        sample::UniformedSample,
    },
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
    let pitch_rate = 0.87;

    let src_buffer = wave_container.uniformed_sample_buffer();
    let window_size = 128usize;
    let window_half = window_size >> 1;
    let window_function = EWindowFunction::Hann;

    let mut dst_buffer = vec![];
    let dst_samples_size = ((src_buffer.len() as f64) / pitch_rate).ceil() as usize;
    dst_buffer.resize(dst_samples_size, UniformedSample::default());

    for n in 0..dst_samples_size {
        let t = pitch_rate * (n as f64);
        let ta = t.floor() as usize;

        let mut tb = 0usize;
        if t == (ta as f64) {
            tb = ta;
        } else {
            tb = ta + 1;
        }

        let hann_src = (tb as isize) - (window_half as isize);
        let hann_dst = (ta as isize) + (window_half as isize);
        let hann_length = hann_dst - hann_src;

        let window_src = if tb >= window_half { tb - window_half } else { 0 };
        let window_dst = (ta + window_half).min(src_buffer.len());
        if window_src < window_dst {
            for m in window_src..window_dst {
                // ここでConvolution。
                // s_d(m)sinc(pi(t-m))

                //let hann_value = window_function.get_factor(hann_length as f64, t - (m as f64));
                let sinc_value = sinc((PI as f64) * (t - (m as f64)));
                //dst_buffer[n] += (hann_value * sinc_value) * src_buffer[m];
                dst_buffer[n] += sinc_value * src_buffer[m];
            }
        }
    }

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
