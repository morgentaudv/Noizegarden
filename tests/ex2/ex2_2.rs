use std::{
    fs,
    io::{self, Write},
};

use itertools::Itertools;
use soundprog::wave::{
    container::WaveContainer,
    setting::{
        EBitsPerSample, EFrequencyItem::Constant, EIntensityControlItem, WaveFormatSetting, WaveSound,
        WaveSoundSetting, WaveSoundSettingBuilder,
    },
};

#[allow(dead_code)]
fn create_sound_settings_fromc4toc5(period: f32) -> Option<Vec<WaveSoundSetting>> {
    // C長調、ド4オクターブの周波数からド5まで作る。
    const C4_FLOAT: f32 = 261.63;
    const D4_FLOAT: f32 = 293.66;
    const E4_FLOAT: f32 = 329.63;
    const F4_FLOAT: f32 = 349.23;
    const G4_FLOAT: f32 = 392.00;
    const A4_FLOAT: f32 = 440.00;
    const B4_FLOAT: f32 = 493.88;
    const C5_FLOAT: f32 = C4_FLOAT * 2f32;
    const FREQUENCIES: [f32; 8] = [C4_FLOAT, D4_FLOAT, E4_FLOAT, F4_FLOAT, G4_FLOAT, A4_FLOAT, B4_FLOAT, C5_FLOAT];

    if period <= 0f32 {
        return None;
    }

    let mut base_setting = WaveSoundSettingBuilder::default();
    base_setting
        .frequency(Constant { frequency: 0.0 })
        .length_sec(period)
        .intensity(1.0f64);

    let mut results = vec![];
    for index in 0..FREQUENCIES.len() {
        const FADE_LENGTH: f64 = 0.1;

        results.push(
            base_setting
                .frequency(Constant {
                    frequency: FREQUENCIES[index] as f64,
                })
                .length_sec(period)
                .intensity_control_items(vec![
                    EIntensityControlItem::Fade {
                        start_time: 0.0,
                        length: FADE_LENGTH,
                        start_factor: 0.0,
                        end_factor: 1.0,
                    },
                    EIntensityControlItem::Fade {
                        start_time: (period as f64) - FADE_LENGTH,
                        length: FADE_LENGTH,
                        start_factor: 1.0,
                        end_factor: 0.0,
                    },
                ])
                .build()
                .unwrap(),
        );
    }

    Some(results)
}

#[test]
fn write_fromc4toc5() {
    const WRITE_FILE_PATH: &'static str = "assets/ex2/ex2_2_4.wav";

    let fmt_setting = WaveFormatSetting {
        samples_per_sec: 44100,
        bits_per_sample: EBitsPerSample::Bits16,
    };
    let sound_settings = create_sound_settings_fromc4toc5(1f32).unwrap();

    // 上の情報から波形を作る。
    // まず[0 ~ u32]までのu32値から量子化bitsに合う値として変換する。
    //
    // 浮動小数点を使わない理由としては、24bitsの場合f64でも精度が落ちる可能性がみられる。
    let buffer = {
        let buffers = sound_settings
            .into_iter()
            .map(|setting| {
                let sound = WaveSound::from_settings(&fmt_setting, &[setting]);

                let mut buffer = vec![];
                for mut fragment in sound.sound_fragments {
                    buffer.append(&mut fragment.buffer)
                }
                buffer
            })
            .collect_vec();

        let mut new_buffer = vec![];
        for mut buffer in buffers {
            new_buffer.append(&mut buffer);
        }
        new_buffer
    };

    //WaveContainer::from_uniformed_sample_buffer(original, buffer);

    //let sound = WaveSound::from_settings(&fmt_setting, &sound_settings);
    //// そして情報をまとめてWaveContainerに書く。
    //let container = WaveContainer::from_wavesound(&sound).unwrap();

    //// ファイルの出力
    //{
    //    let dest_file = fs::File::create(WRITE_FILE_PATH).expect("Could not create 500hz.wav.");
    //    let mut writer = io::BufWriter::new(dest_file);
    //    container.write(&mut writer);
    //    writer.flush().expect("Failed to flush writer.")
    //}
}
