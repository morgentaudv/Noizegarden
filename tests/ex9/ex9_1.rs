use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    container::WaveContainer,
    filter::{self, ESourceFilter},
    setting::{EBitsPerSample, WaveFormatSetting, WaveSound, WaveSoundSetting, WaveSoundSettingBuilder},
};

const C4_FLOAT: f32 = 261.63;
const D4_FLOAT: f32 = 293.66;
const E4_FLOAT: f32 = 329.63;
const F4_FLOAT: f32 = 349.23;
const G4_FLOAT: f32 = 392.00;
const A4_FLOAT: f32 = 440.00;
const B4_FLOAT: f32 = 493.88;
const C5_FLOAT: f32 = C4_FLOAT * 2f32;

fn sawtooth_fragments(startTime: f32, period: f32, frequency: f32, order: u32) -> Option<Vec<WaveSoundSetting>> {
    if startTime < 0f32 || period <= 0f32 {
        return None;
    }

    let mut results = vec![];
    let mut setting = WaveSoundSettingBuilder::default();

    // 基本音を入れる。
    setting
        .frequency(frequency)
        .start_sec(startTime)
        .length_sec(period)
        .intensity(0.4f64);
    results.push(setting.build().unwrap());

    // 倍音を入れる。
    for i in 2..order {
        let overtone_frequency = (frequency * (i as f32));
        let intensity = 0.4f64 * (i as f64).recip();
        results.push(setting.frequency(overtone_frequency).intensity(intensity).build().unwrap());
    }

    Some(results)
}

#[test]
fn test_ex9_1() {
    const WRITE_FILE_PATH: &'static str = "assets/ex9/ex9_1_tremolo.wav";

    let original_sound = {
        let fmt_setting = WaveFormatSetting {
            samples_per_sec: 44100,
            bits_per_sample: EBitsPerSample::Bits16,
        };
        let sound_settings = sawtooth_fragments(0f32, 5f32, C5_FLOAT, 100).unwrap();
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
