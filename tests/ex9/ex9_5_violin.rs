use std::{
    fs,
    io::{self, Write},
};

use soundprog::wave::{
    container::WaveContainer,
    filter::{EEdgeFrequency, EFilter, ESourceFilter, FilterCommonSetting},
    setting::{EBitsPerSample, EFrequencyItem, WaveFormatSetting, WaveSoundBuilder, WaveSoundSettingBuilder},
};

use crate::ex9::C5_FLOAT;

#[test]
fn test_ex9_5_violin() {
    const WRITE_FILE_PATH: &'static str = "assets/ex9/ex9_5_adsr_violin.wav";

    let vco_container = {
        let setting = WaveSoundSettingBuilder::default()
            .length_sec(5.0)
            .frequency(EFrequencyItem::Sawtooth {
                frequency: C5_FLOAT as f64,
            })
            .intensity(0.225)
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

    let vcf_buffer = EFilter::IIRLowPass {
        edge_frequency: EEdgeFrequency::Constant(1500.0),
        quality_factor: 5.0,
    }
    .apply_to_buffer(
        &FilterCommonSetting {
            channel: 1,
            samples_per_second: 44100,
        },
        vco_container.uniformed_sample_buffer(),
    );

    let vca_buffer = ESourceFilter::AmplitudeADSR {
        attack_sample_len: vcf_buffer.len() >> 3,
        decay_sample_len: 0,
        sustain_intensity: 1.0,
        release_sample_len: vcf_buffer.len() >> 3,
        gate_sample_len: vcf_buffer.len() - (vcf_buffer.len() >> 3),
        duration_sample_len: vcf_buffer.len(),
    }
    .apply_to_buffer(&vcf_buffer);

    // ファイルの出力
    {
        let output_container = WaveContainer::from_uniformed_sample_buffer(&vco_container, vca_buffer);

        let dest_file = fs::File::create(WRITE_FILE_PATH).expect("Could not create 500hz.wav.");
        let mut writer = io::BufWriter::new(dest_file);
        output_container.write(&mut writer);
        writer.flush().expect("Failed to flush writer.")
    }
}
