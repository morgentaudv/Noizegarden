use miniaudio::{DeviceType, FramesMut};
use std::sync::{Arc, Mutex, OnceLock, Weak};

/// 24-12-10
/// mutにしているのは、[`AudioDevice::cleanup()`]で値をTakeするため。
/// 他に良い方法があればそれにしてmutをなくしたい。
static mut AUDIO_DEVICE: OnceLock<Arc<Mutex<AudioDevice>>> = OnceLock::new();

/// [`AudioDevice`]を生成するための初期設定のための構造体。
pub struct AudioDeviceConfig {
    /// @brief 初期チャンネル数
    channels: usize,
    /// @brief 初期サンプルレート
    sample_rate: usize,
}

impl AudioDeviceConfig {
    /// デフォルトインスタンスの生成。
    pub fn new() -> Self {
        Self {
            channels: 0,
            sample_rate: 0,
        }
    }

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
    /// システムを初期化する。
    /// すべての処理（レンダリング）が始まる前に処理すべき。
    pub fn initialize(config: AudioDeviceConfig) -> AudioDeviceProxyWeakPtr {
        // @todo 24-12-10 ここら辺のコード、結構危なっかしいのであとでちゃんとしたものに書き換えしたい。
        // こっからProxyを作って、weakを渡してから
        let original_proxy = unsafe {
            assert!(AUDIO_DEVICE.get().is_none());

            // デバイスの初期化
            let device = AUDIO_DEVICE.get_or_init(move || {
                let device = Self::new(config);
                Arc::new(Mutex::new(device))
            });
            let weak_device = Arc::downgrade(&device);

            let original_proxy = AudioDeviceProxy::new(weak_device);
            original_proxy
        };

        // Proxyの登録。
        let weak_proxy = Arc::downgrade(&original_proxy);
        unsafe {
            // Mutexがおそらく内部Internal Mutabilityを実装しているかと。
            let mut instance = AUDIO_DEVICE.get().expect("AudioDevice instance must be valid");
            instance.lock().unwrap().original_proxy = Some(original_proxy);
        }

        // Proxyを返す。本体は絶対返さない。
        weak_proxy
    }

    /// システムの対応。
    pub fn get_proxy() -> Option<AudioDeviceProxyWeakPtr> {
        // これは大丈夫か。。。。
        unsafe {
            match AUDIO_DEVICE.get() {
                None => None,
                Some(v) => Some(Arc::downgrade(v.lock().unwrap().original_proxy.as_ref()?)),
            }
        }
    }

    /// システムを解放する。
    /// すべての関連処理が終わった後に解放すべき。
    pub fn cleanup() {
        unsafe {
            assert!(AUDIO_DEVICE.get().is_some());

            // ここでdropするので、もう1回解放してはいけない。
            if let Some(device) = AUDIO_DEVICE.take() {
                drop(device)
            }
        }
    }

    fn new(config: AudioDeviceConfig) -> Self {
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
    /// 親元となるProxyのマルチスレッド版のインスタンスを生成する。
    fn new(device: Weak<Mutex<AudioDevice>>) -> AudioDeviceProxyPtr {
        let instance = Self { device };
        Arc::new(Mutex::new(instance))
    }
}

type AudioDeviceProxyPtr = Arc<Mutex<AudioDeviceProxy>>;
type AudioDeviceProxyWeakPtr = Weak<Mutex<AudioDeviceProxy>>;

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
