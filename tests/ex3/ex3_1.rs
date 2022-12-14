use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    container::WaveContainer,
    setting::{
        EBitsPerSample, EFrequencyItem, WaveFormatSetting, WaveSound, WaveSoundSetting, WaveSoundSettingBuilder,
    },
};

const C4_FLOAT: f32 = 261.63;
const D4_FLOAT: f32 = 293.66;
const E4_FLOAT: f32 = 329.63;
const F4_FLOAT: f32 = 349.23;
const G4_FLOAT: f32 = 392.00;
const A4_FLOAT: f32 = 440.00;
const B4_FLOAT: f32 = 493.88;
const C5_FLOAT: f32 = C4_FLOAT * 2f32;

fn sawtooth_fragments(period: f32, frequency: f32, order: u32) -> Option<Vec<WaveSoundSetting>> {
    if period <= 0f32 {
        return None;
    }

    let mut results = vec![];
    let mut setting = WaveSoundSettingBuilder::default();

    // 基本音を入れる。
    setting
        .frequency(EFrequencyItem::Constant {
            frequency: frequency as f64,
        })
        .length_sec(period)
        .intensity(0.4f64);
    results.push(setting.build().unwrap());

    // 倍音を入れる。
    for i in 2..order {
        let overtone_frequency = frequency * (i as f32);
        let intensity = 0.4f64 * (i as f64).recip();
        results.push(
            setting
                .frequency(EFrequencyItem::Constant {
                    frequency: overtone_frequency as f64,
                })
                .intensity(intensity)
                .length_sec(period)
                .build()
                .unwrap(),
        );
    }

    Some(results)
}

#[test]
fn write_fromc4toc5() {
    const WRITE_FILE_PATH: &'static str = "assets/ex3/ex3_1.wav";

    let fmt_setting = WaveFormatSetting {
        samples_per_sec: 44100,
        bits_per_sample: EBitsPerSample::Bits16,
    };
    let sound_settings = sawtooth_fragments(1f32, C5_FLOAT, 100).unwrap();

    // 上の情報から波形を作る。
    // まず[0 ~ u32]までのu32値から量子化bitsに合う値として変換する。
    //
    // 浮動小数点を使わない理由としては、24bitsの場合f64でも精度が落ちる可能性がみられる。
    let sound = WaveSound::from_settings(&fmt_setting, &sound_settings);
    // そして情報をまとめてWaveContainerに書く。
    let container = WaveContainer::from_wavesound(&sound).unwrap();

    // ファイルの出力
    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect("Could not create 500hz.wav.");
        let mut writer = io::BufWriter::new(dest_file);
        container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
