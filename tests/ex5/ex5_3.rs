use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    container::WaveContainer,
    setting::{EBitsPerSample, WaveFormatSetting, WaveSound, WaveSoundSetting, WaveSoundSettingBuilder},
};

fn organ_sound(start_time: f32, period: f32, frequency: f64) -> Option<Vec<WaveSoundSetting>> {
    if start_time < 0f32 || period <= 0f32 {
        return None;
    }

    let mut results = vec![];
    let mut setting = WaveSoundSettingBuilder::default();

    let wave_infos = [
        (frequency, 0.5),
        (frequency * 2.0, 1.0),
        (frequency * 3.0, 0.7),
        (frequency * 4.0, 0.5),
        (frequency * 5.0, 0.3),
    ];

    // 基本音を入れる。
    const BASE_INTENSITY: f64 = 0.2;
    setting.start_sec(start_time).length_sec(period);
    for wave_info in wave_infos {
        results.push(
            setting
                .frequency(wave_info.0 as f32)
                .intensity(BASE_INTENSITY * wave_info.1)
                .build()
                .unwrap(),
        );
    }

    Some(results)
}

#[test]
fn ex5_3_test() {
    const WRITE_FILE_PATH: &'static str = "assets/ex5/ex5_3.wav";

    let fmt_setting = WaveFormatSetting {
        samples_per_sec: 44100,
        bits_per_sample: EBitsPerSample::Bits16,
    };
    let sound_settings = organ_sound(0f32, 5f32, 440.0).unwrap();
    let sound = WaveSound::from_settings(&fmt_setting, &sound_settings);
    let container = WaveContainer::from_wavesound(&sound).unwrap();

    // ファイルの出力
    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect("Could not create file.");
        let mut writer = io::BufWriter::new(dest_file);
        container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
