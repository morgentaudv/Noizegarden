use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    container::WaveContainer,
    setting::{EBitsPerSample, EFrequencyItem, WaveFormatSetting, WaveSoundBuilder, WaveSoundSettingBuilder},
};

#[test]
fn test_ex10_2_fm() {
    const WRITE_FILE_PATH: &'static str = "assets/ex10/ex10_2_fm.wav";

    let container = {
        let length: f64 = 2.0;
        let setting = WaveSoundSettingBuilder::default()
            .length_sec(length as f32)
            .frequency(EFrequencyItem::FreqModulation {
                carrier_amp: 1.0,
                carrier_freq: 500.0,
                modulator_amp: 1.0,
                freq_ratio: 2.0,
            })
            .intensity(0.5)
            .build()
            .unwrap();

        let fmt_setting = WaveFormatSetting {
            samples_per_sec: 44100,
            bits_per_sample: EBitsPerSample::Bits16,
        };

        let wave_sound = WaveSoundBuilder {
            format: fmt_setting,
            sound_settings: vec![setting],
        }
        .into_build();

        WaveContainer::from_wavesound(&wave_sound).unwrap()
    };

    // ファイルの出力
    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect("Could not create 500hz.wav.");
        let mut writer = io::BufWriter::new(dest_file);
        container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
