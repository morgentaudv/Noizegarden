use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{container::WaveContainer, sample::UniformedSample};

#[test]
fn ex11_1() {
    //const READ_FILE_PATH: &'static str = "assets/ex11/sine_2s.wav";
    const READ_FILE_PATH: &'static str = "assets/ex6/drum.wav";
    const WRITE_FILE_PATH: &'static str = "assets/ex11/ex11_1_output.wav";
    const STRETCH_RATE: f64 = 10.0;
    assert!(STRETCH_RATE > 0.5);

    let wave_container = {
        let source_file = fs::File::open(READ_FILE_PATH).expect(&format!("Could not find {}.", READ_FILE_PATH));
        let mut reader = io::BufReader::new(source_file);

        WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
    };

    // まず音源から周期性を把握する。
    let original_buffer = wave_container.uniformed_sample_buffer();
    let original_samples_size = original_buffer.len();

    let original_fs = wave_container.samples_per_second();
    let processed_samples_size = {
        let samples_size = original_samples_size as f64 / STRETCH_RATE as f64;
        samples_size.ceil() as usize
    };
    let template_size = (original_fs as f64 * 0.01) as usize;
    let p_min = (original_fs as f64 * 0.005) as usize;
    let p_max = (original_fs as f64 * 0.02) as usize;

    let mut x_buffer = vec![];
    x_buffer.resize(template_size, UniformedSample::default());
    let mut y_buffer = vec![];
    y_buffer.resize(template_size, UniformedSample::default());
    let mut r_buffer = vec![];
    r_buffer.resize(p_max + 1, UniformedSample::default());

    let mut out_buffer = vec![];
    out_buffer.resize(processed_samples_size, UniformedSample::default());

    let mut offset0 = 0;
    let mut offset1 = 0;
    while (offset0 + (p_max * 2)) < original_samples_size {
        {
            let start_i = offset0;
            let end_i = start_i + template_size;
            x_buffer.copy_from_slice(&original_buffer[start_i..end_i]);
        }

        // mサンプルずらして記入
        let mut r_max = 0.0;
        let mut period = p_min;
        for m in p_min..=p_max {
            {
                let start_i = offset0 + m;
                let end_i = start_i + template_size;
                y_buffer.copy_from_slice(&original_buffer[start_i..end_i]);
            }

            let mut result = UniformedSample::default();
            for n in 0..template_size {
                //let y_index = offset0 + m + n;
                result += x_buffer[n] * y_buffer[n];
            }

            if result.to_f64() > r_max {
                r_max = result.to_f64();
                period = m;
            }

            r_buffer[m] = result;
        }

        for n in 0..period {
            // 単調減少の重み付け。Lerpっぽくする。
            let b_factor = (n as f64) / (period as f64);
            let a_factor = 1.0 - b_factor;
            out_buffer[offset1 + n] = a_factor * original_buffer[offset0 + n];
            out_buffer[offset1 + n] += b_factor * original_buffer[offset0 + period + n];
        }

        let q_param = ((period as f64) / (STRETCH_RATE - 1.0)).round() as usize;
        for n in period..q_param {
            if offset0 + period + n >= original_samples_size {
                break;
            }

            out_buffer[offset1 + n] = original_buffer[offset0 + period + n];
        }

        offset0 += period + q_param;
        offset1 += q_param;

        println!("{}, {}, {}, {}", offset0, offset1, period, q_param);
    }

    let write_container = WaveContainer::from_uniformed_sample_buffer(&wave_container, out_buffer);
    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect(&format!("Could not create {}.", WRITE_FILE_PATH));
        let mut writer = io::BufWriter::new(dest_file);
        write_container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
