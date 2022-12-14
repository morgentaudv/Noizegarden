use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    container::WaveContainer,
    psg::EPSGSignal,
    setting::{EBitsPerSample, OscillatorVibrato, WaveFormatSetting, WaveSoundBuilder},
    Second,
};

use crate::ex9::C5_FLOAT;

#[test]
fn test_ex9_2() {
    const WRITE_FILE_PATH: &'static str = "assets/ex9/ex9_2_vibrato.wav";

    let original_sound = {
        let fmt_setting = WaveFormatSetting {
            samples_per_sec: 44100,
            bits_per_sample: EBitsPerSample::Bits16,
        };
        let sound_settings = EPSGSignal::Sawtooth {
            length_time: Second(5.0),
            frequency: C5_FLOAT as f64,
            order: 100,
            intensity: 0.1,
        }
        .apply()
        .unwrap();

        WaveSoundBuilder {
            format: fmt_setting,
            sound_settings,
            oscillator_vibrator: Some(OscillatorVibrato {
                initial_frequency: C5_FLOAT as f64,
                period_scale_factor: 100.0,
                periodic_frequency: 2.0,
            }),
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
