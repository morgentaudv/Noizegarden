use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    container::WaveContainer,
    setting::{EBitsPerSample, EFrequencyItem::Constant, WaveFormatSetting, WaveSound, WaveSoundSettingBuilder},
};

#[test]
fn write_500hz_1second() {
    const WRITE_FILE_PATH: &'static str = "assets/ex2/500hz.wav";

    let fmt_setting = WaveFormatSetting {
        samples_per_sec: 44100,
        bits_per_sample: EBitsPerSample::Bits16,
    };
    let sound_setting = WaveSoundSettingBuilder::default()
        .frequency(Constant { frequency: 500.0 })
        .length_sec(1f32)
        .intensity(1.0f64)
        .build()
        .unwrap();

    // 上の情報から波形を作る。
    // まず[0 ~ u32]までのu32値から量子化bitsに合う値として変換する。
    //
    // 浮動小数点を使わない理由としては、24bitsの場合f64でも精度が落ちる可能性がみられる。
    let sound = WaveSound::from_setting(&fmt_setting, &sound_setting);
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
