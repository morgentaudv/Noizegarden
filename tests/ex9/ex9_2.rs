use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    container::WaveContainer,
    setting::{
        EBitsPerSample, EFrequencyItem, OscillatorVibrato, WaveFormatSetting, WaveSoundBuilder, WaveSoundSettingBuilder,
    },
};

use crate::ex9::C5_FLOAT;

#[test]
fn test_ex9_2() {
    const WRITE_FILE_PATH: &'static str = "assets/ex9/ex9_2_vibrato.wav";

    let original_sound = {
        let setting = WaveSoundSettingBuilder::default()
            .length_sec(5.0)
            .frequency(EFrequencyItem::Sawtooth {
                frequency: C5_FLOAT as f64,
            })
            .oscillator_vibrato(Some(OscillatorVibrato {
                period_scale_factor: 300.0,
                periodic_frequency: 4.0,
            }))
            .intensity(0.25)
            .build()
            .unwrap();

        let fmt_setting = WaveFormatSetting {
            samples_per_sec: 44100,
            bits_per_sample: EBitsPerSample::Bits16,
        };

        WaveSoundBuilder {
            format: fmt_setting,
            sound_settings: vec![setting],
        }
        .into_build()
    };

    let new_sound_container = WaveContainer::from_wavesound(&original_sound).unwrap();

    // ファイルの出力
    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect("Could not create 500hz.wav.");
        let mut writer = io::BufWriter::new(dest_file);
        new_sound_container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
