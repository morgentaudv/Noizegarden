use miniaudio::{Device, DeviceConfig, DeviceType, Format, Waveform, WaveformConfig, WaveformType};
use crate::miniaudio::wait_for_enter;

pub const DEVICE_FORMAT: Format = Format::F32;
pub const DEVICE_CHANNELS: u32 = 2;
pub const DEVICE_SAMPLE_RATE: u32 = miniaudio::SAMPLE_RATE_48000;

#[test]
fn test_miniaudio_playback_sine() {
    // WaveformのフォーマットはOutputと同じにする必要がある。
    let sine_wave_config = WaveformConfig::new(
        DEVICE_FORMAT,
        DEVICE_CHANNELS,
        DEVICE_SAMPLE_RATE,
        WaveformType::Sine,
        0.2,
        440.0
    );
    let mut sine_wave = Waveform::new(&sine_wave_config);

    let mut device_config = DeviceConfig::new(DeviceType::Playback);
    device_config.playback_mut().set_format(DEVICE_FORMAT);
    device_config.playback_mut().set_channels(DEVICE_CHANNELS);
    device_config.set_sample_rate(DEVICE_SAMPLE_RATE);

    // デバイスの設定が変わるとストップされるらしい。
    // 理想的にはストップコールバックから通知して、デバイスを作り直す。というのが望ましいかと。
    // デバイスが作り直せなかったらサイレントにするとか。クラッシュはさけたいかな。。
    // (↑) 別のテストで実装してみる。
    device_config.set_stop_callback(|_| {
       println!("Stopped");
    });
    device_config.set_data_callback(move |_device, output, input| {
        sine_wave.read_pcm_frames(output);
    });

    let device = Device::new(None, &device_config).expect("Failed to open device");
    device.start().expect("Failed to start device");

    println!("Device backend: {:?}", device.context().backend());
    println!("Device Sample Rate: {}Hz", device.sample_rate());
    wait_for_enter();

}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
