use std::{
    f32::consts::PI,
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    container::WaveContainer,
    setting::{
        EBitsPerSample, WaveFormatSetting, WaveSound, WaveSoundSetting, WaveSoundSettingBuilder,
    },
};

const C4_FLOAT: f32 = 261.63;
const C5_FLOAT: f32 = C4_FLOAT * 2f32;

fn sawtooth_cosine_fragments(
    startTime: f32,
    period: f32,
    frequency: f32,
    order: u32,
) -> Option<Vec<WaveSoundSetting>> {
    if startTime < 0f32 || period <= 0f32 {
        return None;
    }

    let mut results = vec![];
    let mut setting = WaveSoundSettingBuilder::default();

    // 基本音を入れる。
    const BASE_INTENSITY: f64 = 0.2;
    setting
        .frequency(frequency)
        .phase(PI / 2.0)
        .start_sec(startTime)
        .length_sec(period)
        .intensity(BASE_INTENSITY);
    results.push(setting.build().unwrap());

    // 倍音を入れる。
    for i in 2..order {
        let overtone_frequency = frequency * (i as f32);
        let intensity = BASE_INTENSITY * (i as f64).recip();

        results.push(
            setting
                .frequency(overtone_frequency)
                .intensity(intensity)
                .build()
                .unwrap(),
        );
    }

    Some(results)
}

#[test]
fn write_fromc4toc5() {
    const WRITE_FILE_PATH: &'static str = "assets/ex3/ex3_4.wav";

    let fmt_setting = WaveFormatSetting {
        samples_per_sec: 44100,
        bits_per_sample: EBitsPerSample::Bits16,
    };
    let sound_settings = sawtooth_cosine_fragments(0f32, 1f32, C5_FLOAT, 50).unwrap();
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
