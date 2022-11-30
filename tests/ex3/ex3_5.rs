use std::{
    f32::consts::PI,
    fs,
    io::{self, Write},
};

use rand::prelude::*;
use soundprog::wave::{
    container::WaveContainer,
    setting::{EBitsPerSample, WaveFormatSetting, WaveSound, WaveSoundSetting, WaveSoundSettingBuilder},
};

const C4_FLOAT: f32 = 261.63;
const C5_FLOAT: f32 = C4_FLOAT * 2f32;

fn whitenoise_fragments(startTime: f32, period: f32) -> Option<Vec<WaveSoundSetting>> {
    if startTime < 0f32 || period <= 0f32 {
        return None;
    }

    let mut results = vec![];
    let mut setting = WaveSoundSettingBuilder::default();

    // 基本音を入れる。
    const BASE_INTENSITY: f64 = 0.01;
    const FREQ_RANGE: f32 = 20000.0;
    setting
        .frequency(0.0)
        .phase(0.0)
        .start_sec(startTime)
        .length_sec(period)
        .intensity(BASE_INTENSITY);
    results.push(setting.build().unwrap());

    // 倍音を入れる。
    let mut rng = rand::thread_rng();
    for i in 0..250 {
        let frequency = (rng.gen::<f32>() * FREQ_RANGE);
        let phase = rng.gen::<f32>() * (PI * 2.0);

        results.push(setting.frequency(frequency).phase(phase).build().unwrap());
    }

    Some(results)
}

#[test]
fn write_fromc4toc5() {
    const WRITE_FILE_PATH: &'static str = "assets/ex3/ex3_5.wav";

    let fmt_setting = WaveFormatSetting {
        samples_per_sec: 44100,
        bits_per_sample: EBitsPerSample::Bits16,
    };
    let sound_settings = whitenoise_fragments(0f32, 1f32).unwrap();
    let sound = WaveSound::from_settings(&fmt_setting, &sound_settings);
    let container = WaveContainer::from_wavesound(&sound).unwrap();

    // ファイルの出力
    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect("Could not create 500hz.wav.");
        let mut writer = io::BufWriter::new(dest_file);
        container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
