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
    const STRETCH_RATE: f64 = 3.0;
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
    let template_size = (original_fs as f64 * 0.01) as usize;
    let p_min = (original_fs as f64 * 0.005) as usize;
    let p_max = (original_fs as f64 * 0.02) as usize;

    let mut out_proceeded_endi = 0;
    let mut out_buffer = vec![];
    out_buffer.resize(1, UniformedSample::default());

    let mut offset_src = 0;
    let mut offset_dst = 0;
    while (offset_src + (p_max * 2)) < original_samples_size {
        // mサンプルずらして記入。一種のConvolutionを行う。
        // 一番Peak(正の数)が波形の周期だとみなす。
        let mut r_max = 0.0;
        let mut period = p_min;
        for m in p_min..=p_max {
            let mut result = 0.0;
            for n in 0..template_size {
                let x_index = offset_src + n;
                let y_index = offset_src + m + n;
                result += original_buffer[x_index].to_f64() * original_buffer[y_index].to_f64();
            }

            if result > r_max {
                r_max = result;
                period = m;
            }
        }

        // 元アルゴリズムとは違ってバッファを動的に用意する。
        // 複雑な波形の場合には固定のバッファだと枠がたりなくなる。
        out_proceeded_endi = offset_dst + period;
        if out_proceeded_endi >= out_buffer.len() {
            let resize_len = out_proceeded_endi.next_power_of_two();
            out_buffer.resize(resize_len, UniformedSample::default());
        }

        for n in 0..period {
            // 単調減少の重み付け。Lerpっぽくする。
            let b_factor = (n as f64) / (period as f64);
            let a_factor = 1.0 - b_factor;
            out_buffer[offset_dst + n] = a_factor * original_buffer[offset_src + n];
            out_buffer[offset_dst + n] += b_factor * original_buffer[offset_src + period + n];
        }

        let q_param = ((period as f64) / (STRETCH_RATE - 1.0)).round() as usize;
        for n in period..q_param {
            if offset_src + period + n >= original_samples_size {
                break;
            }

            out_buffer[offset_dst + n] = original_buffer[offset_src + period + n];
        }

        offset_src += period + q_param;
        offset_dst += q_param;
    }
    let _ = out_buffer.split_off(out_proceeded_endi);

    let write_container = WaveContainer::from_uniformed_sample_buffer(&wave_container, out_buffer);
    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect(&format!("Could not create {}.", WRITE_FILE_PATH));
        let mut writer = io::BufWriter::new(dest_file);
        write_container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
