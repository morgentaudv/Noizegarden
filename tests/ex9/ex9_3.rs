use std::{
    f64::consts::PI,
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    container::WaveContainer,
    filter::{self, EEdgeFrequency, EFilter},
    setting::{
        EBitsPerSample, EFrequencyItem, OscillatorVibrato, WaveFormatSetting, WaveSoundBuilder, WaveSoundSettingBuilder,
    },
};

use crate::ex9::C5_FLOAT;

#[test]
fn test_ex9_3() {
    const WRITE_FILE_PATH: &'static str = "assets/ex9/ex9_3_wow.wav";

    let original_sound = {
        let setting = WaveSoundSettingBuilder::default()
            .length_sec(5.0)
            .frequency(EFrequencyItem::Sawtooth {
                frequency: C5_FLOAT as f64,
            })
            .intensity(0.5)
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
    let new_container = EFilter::IIRLowPass {
        edge_frequency: EEdgeFrequency::ChangeBySample(|sample_i, _, samples_per_sec| {
            // LFOによるWOW効果を真似する。
            const INITIAL_FREQUENCY: f64 = 1000.0;
            const SCALE: f64 = 800.0;
            const FREQUENCY: f64 = 2.0;

            let rel_time = (sample_i as f64) / (samples_per_sec as f64);
            INITIAL_FREQUENCY + (SCALE * (PI * 2.0 * rel_time * FREQUENCY).sin())
        }),
        quality_factor: 5f64.sqrt().recip(),
        adsr: None,
    }
    .apply_to_wave_container(&new_sound_container);

    // ファイルの出力
    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect("Could not create 500hz.wav.");
        let mut writer = io::BufWriter::new(dest_file);
        new_container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
