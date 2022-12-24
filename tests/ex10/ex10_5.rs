use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    container::WaveContainer,
    setting::{
        EBitsPerSample, EFrequencyItem, WaveFormatSetting, WaveSoundADSR, WaveSoundBuilder, WaveSoundSettingBuilder,
    },
};

#[test]
fn test_ex10_5_fm_electric_piano() {
    const WRITE_FILE_PATH: &'static str = "assets/ex10/ex10_5_fm_electric_piano.wav";

    let length: f64 = 4.0;
    let fmt_setting = WaveFormatSetting {
        samples_per_sec: 44100,
        bits_per_sample: EBitsPerSample::Bits16,
    };

    let container = {
        let diminish = WaveSoundSettingBuilder::default()
            .length_sec(length as f32)
            .frequency(EFrequencyItem::FreqModulation {
                carrier_amp: 1.0,
                carrier_freq: 440.0,
                modulator_amp: 1.0,
                freq_ratio: 1.0,
                carrier_amp_adsr: Some(WaveSoundADSR {
                    attack_len_second: 0.0,
                    decay_len_second: length,
                    sustain_intensity: 0.0,
                    release_len_second: 0.0,
                    gate_len_second: length,
                    duration_len_second: length,
                    process_fn: |orig, intensity| orig * intensity,
                }),
                modulator_amp_adsr: Some(WaveSoundADSR {
                    attack_len_second: 0.0,
                    decay_len_second: length * 0.5,
                    sustain_intensity: 0.0,
                    release_len_second: length * 0.5,
                    gate_len_second: length,
                    duration_len_second: length,
                    process_fn: |orig, intensity| orig * intensity,
                }),
            })
            .intensity(0.25)
            .build()
            .unwrap();
        let percussion = WaveSoundSettingBuilder::default()
            .length_sec(length as f32)
            .frequency(EFrequencyItem::FreqModulation {
                carrier_amp: 1.0,
                carrier_freq: 440.0,
                modulator_amp: 1.0,
                freq_ratio: 14.0,
                carrier_amp_adsr: Some(WaveSoundADSR {
                    attack_len_second: 0.0,
                    decay_len_second: length * 0.25,
                    sustain_intensity: 0.0,
                    release_len_second: length * 0.25,
                    gate_len_second: length,
                    duration_len_second: length,
                    process_fn: |orig, intensity| orig * intensity,
                }),
                modulator_amp_adsr: Some(WaveSoundADSR {
                    attack_len_second: 0.0,
                    decay_len_second: length * 0.25,
                    sustain_intensity: 0.0,
                    release_len_second: length * 0.25,
                    gate_len_second: length,
                    duration_len_second: length,
                    process_fn: |orig, intensity| orig * intensity,
                }),
            })
            .intensity(0.25)
            .build()
            .unwrap();

        let wave_sound = WaveSoundBuilder {
            format: fmt_setting,
            sound_settings: vec![diminish, percussion],
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
