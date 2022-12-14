use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    container::WaveContainer,
    setting::{
        EBitsPerSample, EFrequencyItem, EIntensityControlItem, WaveFormatSetting, WaveSound, WaveSoundSetting,
        WaveSoundSettingBuilder,
    },
};

fn piano_sound(period: f32, frequency: f64) -> Option<Vec<WaveSoundSetting>> {
    if period <= 0f32 {
        return None;
    }

    let mut results = vec![];
    let mut setting = WaveSoundSettingBuilder::default();

    let wave_infos = [
        (frequency, 1.0, 4.0),
        (frequency * 2.0, 0.8, 2.0),
        (frequency * 3.0, 0.6, 1.0),
        (frequency * 4.0, 0.5, 0.5),
        (frequency * 5.0, 0.4, 0.2),
    ];

    // 基本音を入れる。
    const BASE_INTENSITY: f64 = 0.2;
    setting.length_sec(period);
    for (frequency, intensity, exp_recip) in wave_infos {
        results.push(
            setting
                .frequency(EFrequencyItem::Constant { frequency })
                .intensity(BASE_INTENSITY * intensity)
                .intensity_control_items(vec![EIntensityControlItem::Exp {
                    start_time: 0.0,
                    length: None,
                    coefficient: -5.0 / exp_recip,
                }])
                .build()
                .unwrap(),
        );
    }

    Some(results)
}

#[test]
fn ex5_4_test() {
    const WRITE_FILE_PATH: &'static str = "assets/ex5/ex5_4.wav";

    let fmt_setting = WaveFormatSetting {
        samples_per_sec: 44100,
        bits_per_sample: EBitsPerSample::Bits16,
    };
    let sound_settings = piano_sound(5f32, 440.0).unwrap();
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
