use miniaudio::{DeviceType, FramesMut};
use std::sync::{Arc, Mutex, OnceLock, Weak};

static AUDIO_DEVICE: OnceLock<Arc<Mutex<AudioDevice>>> = OnceLock::new();

pub struct AudioDeviceConfig {
    /// @brief 初期チャンネル数
    channels: usize,
    /// @brief 初期サンプルレート
    sample_rate: usize,
}

impl AudioDeviceConfig {
    /// @brief 初期チャンネル数の指定
    pub fn set_channels(&mut self, channels: usize) -> &mut Self {
        self.channels = channels;
        self
    }

    /// @brief 初期サンプルレートの指定
    pub fn set_sample_rate(&mut self, sample_rate: usize) -> &mut Self {
        self.sample_rate = sample_rate;
        self
    }
}

pub struct AudioDevice {
    /// @brief ローレベルのデバイス
    low_device: miniaudio::Device,
    /// @brief プロキシの親元。ほかのところでは全部Weakタイプで共有する。
    original_proxy: Option<AudioDeviceProxyPtr>,
}

impl AudioDevice {
    pub fn new(config: AudioDeviceConfig) -> Self {
        assert!(config.channels > 0);
        assert!(config.sample_rate > 0);

        let mut low_device_config = miniaudio::DeviceConfig::new(DeviceType::Playback);
        low_device_config.playback_mut().set_format(miniaudio::Format::F32);
        low_device_config.playback_mut().set_channels(config.channels as u32);
        low_device_config.set_sample_rate(config.sample_rate as u32);
        low_device_config.set_data_callback(Self::on_update_device_callback);
        low_device_config.set_stop_callback(Self::on_stop_device_callback);

        let low_device = miniaudio::Device::new(None, &low_device_config).expect("failed to create audio device");
        Self {
            low_device,
            original_proxy: None, // これはあとで初期化する。
        }
    }

    pub fn initialize(config: AudioDeviceConfig) -> AudioDeviceProxyWeakPtr {
        assert!(AUDIO_DEVICE.get().is_none());

        let device = AUDIO_DEVICE.get_or_init(move || {
            let device = Self::new(config);
            Arc::new(Mutex::new(device))
        });

        let weak_device = Arc::downgrade(&device);
        todo!()
    }

    fn on_update_device_callback(_device: &miniaudio::RawDevice, _output: &mut FramesMut, _input: &miniaudio::Frames) {}

    fn on_stop_device_callback(_device: &miniaudio::RawDevice) {}
}

unsafe impl Sync for AudioDevice {}

impl Drop for AudioDevice {
    fn drop(&mut self) {
        todo!()
    }
}

/// レンダリングアイテムからデバイスに接近するためのプロキシ。
pub struct AudioDeviceProxy {
    /// @brief デバイスに接近するためのもの。
    device: Weak<Mutex<AudioDevice>>,
}

impl AudioDeviceProxy {
    fn new(device: Weak<Mutex<AudioDevice>>) -> Self {
        Self { device }
    }
}

type AudioDeviceProxyPtr = Arc<Mutex<AudioDevice>>;
type AudioDeviceProxyWeakPtr = Weak<Mutex<AudioDeviceProxy>>;

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
