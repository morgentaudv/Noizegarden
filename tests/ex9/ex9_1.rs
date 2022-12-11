use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    container::WaveContainer,
    filter::ESourceFilter,
    setting::{EBitsPerSample, WaveFormatSetting, WaveSound, WaveSoundSetting, WaveSoundSettingBuilder},
};

use crate::ex9::C5_FLOAT;

#[repr(transparent)]
pub struct Second(pub f64);

pub enum EPSGSignal {
    Sawtooth {
        length_time: Second,
        frequency: f64,
        order: usize,
    },
}

pub struct OscillatorVibrato {
    initial_frequency: f64,
    period_scale_factor: f64,
    periodic_frequency: f64,
}

impl EPSGSignal {
    pub fn apply(&self) -> Option<Vec<WaveSoundSetting>> {
        match &self {
            EPSGSignal::Sawtooth {
                length_time,
                frequency,
                order,
            } => {
                if length_time.0 <= 0.0 {
                    return None;
                }

                let mut results = vec![];
                let mut setting = WaveSoundSettingBuilder::default();

                // 基本音を入れる。
                setting
                    .frequency(*frequency as f32)
                    .length_sec(length_time.0 as f32)
                    .intensity(0.4f64);
                results.push(setting.build().unwrap());

                // 倍音を入れる。
                for order_i in 2..*order {
                    let overtone_frequency = (*frequency * (order_i as f64));
                    let intensity = 0.4f64 * (order_i as f64).recip();
                    results.push(
                        setting
                            .frequency(overtone_frequency as f32)
                            .intensity(intensity)
                            .build()
                            .unwrap(),
                    );
                }

                Some(results)
            }
        }
    }
}

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
        WaveSound::from_settings(&fmt_setting, &sound_settings)
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
