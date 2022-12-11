use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    container::WaveContainer,
    filter::ESourceFilter,
    psg::EPSGSignal,
    setting::{EBitsPerSample, WaveFormatSetting, WaveSound, WaveSoundBuilder},
    Second,
};

use crate::ex9::C5_FLOAT;

#[test]
fn test_ex9_1() {
    const WRITE_FILE_PATH: &'static str = "assets/ex9/ex9_1_tremolo.wav";

    let original_sound = {
        let fmt_setting = WaveFormatSetting {
            samples_per_sec: 44100,
            bits_per_sample: EBitsPerSample::Bits16,
        };
        let sound_settings = EPSGSignal::Sawtooth {
            length_time: Second(5.0),
            frequency: C5_FLOAT as f64,
            order: 100,
        }
        .apply()
        .unwrap();

        WaveSoundBuilder {
            format: fmt_setting,
            sound_settings,
            oscillator_vibrator: None,
        }
        .into_build()
    };

    // Apply LFO to Amplifier to be tremolo.
    let filtered_buffer = ESourceFilter::AmplitudeTremolo {
        initial_scale: 0.75,
        periodical_scale_factor: 0.25,
        period_time_frequency: 1.0,
        source_samples_per_second: original_sound.format.samples_per_sec as f64,
    }
    .apply_to_buffer(&original_sound.get_completed_samples());

    let new_sound_container = {
        // そして情報をまとめてWaveContainerに書く。
        let container = WaveContainer::from_wavesound(&original_sound).unwrap();
        WaveContainer::from_uniformed_sample_buffer(&container, filtered_buffer)
    };

    // ファイルの出力
    {
        let dest_file = fs::File::create(WRITE_FILE_PATH).expect("Could not create 500hz.wav.");
        let mut writer = io::BufWriter::new(dest_file);
        new_sound_container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
