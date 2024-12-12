use crate::wave::sample::UniformedSample;
use itertools::Itertools;
use miniaudio::{DeviceType, FramesMut, RingBufferRecv, RingBufferSend};
use std::sync::{Arc, Mutex, OnceLock, Weak};

/// 24-12-10
/// mutにしているのは、[`AudioDevice::cleanup()`]で値をTakeするため。
/// 他に良い方法があればそれにしてmutをなくしたい。
static mut AUDIO_DEVICE: OnceLock<Arc<Mutex<AudioDevice>>> = OnceLock::new();

/// コールバックから取得する必要があるので、[`AudioDevice`]には入れない。
/// リングバッファのレシーバー
static mut BUFFER_RECEIVER: OnceLock<Option<Mutex<RingBufferRecv<f32>>>> = OnceLock::new();

/// [`AudioDevice`]を生成するための初期設定のための構造体。
pub struct AudioDeviceConfig {
    /// 初期チャンネル数
    channels: usize,
    /// 初期サンプルレート
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

    /// 初期チャンネル数の指定
    pub fn set_channels(&mut self, channels: usize) -> &mut Self {
        self.channels = channels;
        self
    }

    /// 初期サンプルレートの指定
    pub fn set_sample_rate(&mut self, sample_rate: usize) -> &mut Self {
        self.sample_rate = sample_rate;
        self
    }
}

pub struct AudioDevice {
    /// ローレベルのデバイス
    low_device: miniaudio::Device,
    /// プロキシの親元。ほかのところでは全部Weakタイプで共有する。
    original_proxy: Option<AudioDeviceProxyPtr>,
}

impl AudioDevice {
    /// システムを初期化する。
    /// すべての処理（レンダリング）が始まる前に処理すべき。
    pub fn initialize(config: AudioDeviceConfig) -> AudioDeviceProxyWeakPtr {
        // RingBufferの登録。
        // RingBufferのタイプはf32にして、あとで受け取る側でいい変換して送る。
        //
        // ただし生成したままではちゃんと扱えないので、sendだけはArc<Mutex<>>にはさむ。
        let (send, recv) = miniaudio::ring_buffer::<f32>(1024, 16).expect("Failed to create audio ring buffer.");
        unsafe {
            let _result = BUFFER_RECEIVER.set(Some(Mutex::new(recv)));
        }

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

            let original_proxy = AudioDeviceProxy::new(weak_device, send);
            original_proxy
        };

        // Proxyの登録。
        let weak_proxy = Arc::downgrade(&original_proxy);
        unsafe {
            // Mutexがおそらく内部Internal Mutabilityを実装しているかと。
            let mut instance = AUDIO_DEVICE.get().expect("AudioDevice instance must be valid");
            let mut accessor = instance.lock().unwrap();

            accessor.original_proxy = Some(original_proxy);
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

    /// Tick関数。
    pub fn process(_frame_time: f64) {}

    /// システムを解放する。
    /// すべての関連処理が終わった後に解放すべき。
    pub fn cleanup() {
        unsafe {
            assert!(AUDIO_DEVICE.get().is_some());

            // ここでdropするので、もう1回解放してはいけない。
            if let Some(device) = AUDIO_DEVICE.take() {
                drop(device)
            }

            // Receiverも解放する。
            BUFFER_RECEIVER.take();
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

    /// `frame_time`から現在の設定からの推定の各チャンネルに必要な推定のサンプル数を返す。
    fn get_required_samples(&self, _frame_time: f64) -> usize {
        // @todo まず固定にして動いたら変動させてみる。
        1024
    }

    /// 今デバイスに設定しているチャンネルの数を返す。
    /// もしデバイスが無効になっているのであれば、`0`を返す。
    pub fn get_channels(&self) -> usize {
        self.low_device.playback().channels() as usize
    }

    fn on_update_device_callback(device: &miniaudio::RawDevice, output: &mut FramesMut, _input: &miniaudio::Frames) {
        unsafe {
            debug_assert!(BUFFER_RECEIVER.get().is_some());
        }

        match device.playback().format() {
            miniaudio::Format::S16 => {
                let outputs = output.as_samples_mut::<i16>();
                // f32は[-1, 1]までに。
                let mut raw_samples = vec![];
                raw_samples.resize(outputs.len(), 0.0f32);

                // できるだけ読み切る。
                let mut read_count = 0;
                let mut attempts = 0;
                while read_count < outputs.len() && attempts < 8 {
                    read_count += unsafe {
                        BUFFER_RECEIVER
                            .get()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .lock()
                            .unwrap()
                            .read(&mut raw_samples[read_count..])
                    };
                    attempts += 1;
                }

                // raw_samplesをoutputに変換する。
                for (i, sample) in raw_samples.iter().enumerate() {
                    outputs[i] = UniformedSample::from_f64(*sample as f64).to_16bits();
                }

                // If we're starved, just repeat the last sample on all channels:
                (&mut outputs[read_count..]).iter_mut().for_each(|s| *s = 0);
            }
            miniaudio::Format::F32 => {
                // f32 → f32なので、そのままにしてもいい。
                let outputs = output.as_samples_mut::<f32>();

                // Here we try reading at most 8 subbuffers to attempt to read enough outputs to
                // fill the playback output buffer. We don't allow infinite attempts because we can't be
                // sure how long that would take.
                let mut read_count = 0;
                let mut attempts = 0;
                while read_count < outputs.len() && attempts < 8 {
                    read_count += unsafe {
                        BUFFER_RECEIVER
                            .get()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .lock()
                            .unwrap()
                            .read(&mut outputs[read_count..])
                    };
                    attempts += 1;
                }

                // If we're starved, just repeat the last sample on all channels:
                (&mut outputs[read_count..]).iter_mut().for_each(|s| *s = 0.0);
            }
            _ => unreachable!(),
        }
    }

    fn on_stop_device_callback(_device: &miniaudio::RawDevice) {}
}

unsafe impl Sync for AudioDevice {}

impl Drop for AudioDevice {
    fn drop(&mut self) {
        self.low_device.stop().expect("TODO: panic message");
    }
}

/// レンダリングアイテムからデバイスに接近するためのプロキシ。
pub struct AudioDeviceProxy {
    /// デバイスに接近するためのもの。
    device: Weak<Mutex<AudioDevice>>,
    /// 最終オーディオレンダリングに使うためのリングバッファ。
    buffer_sender: RingBufferSend<f32>,
}

impl AudioDeviceProxy {
    /// 親元となるProxyのマルチスレッド版のインスタンスを生成する。
    fn new(device: Weak<Mutex<AudioDevice>>, buffer_sender: RingBufferSend<f32>) -> AudioDeviceProxyPtr {
        let instance = Self { device, buffer_sender };
        Arc::new(Mutex::new(instance))
    }

    /// `frame_time`から現在の設定からの推定の各チャンネルに必要な推定のサンプル数を返す。
    pub fn get_required_samples(&self, frame_time: f64) -> usize {
        debug_assert!(self.device.upgrade().is_some());

        let device = self.device.upgrade().unwrap();
        let device = device.lock().unwrap();
        device.get_required_samples(frame_time)
    }

    /// 今デバイスに設定しているチャンネルの数を返す。
    /// もしデバイスが無効になっているのであれば、`0`を返す。
    pub fn get_channels(&self) -> usize {
        match self.device.upgrade() {
            None => 0,
            Some(v) => v.lock().unwrap().get_channels(),
        }
    }

    /// デバイスの設定に合わせて適切にサンプルを送信する。
    pub fn send_sample_buffer(&self, requiring_samples: usize, channel_buffers: EDrainedChannelBuffers) {
        let channels = self.get_channels();
        if channels == 0 {
            return;
        }

        // 返されるbufferで設定した各チャンネルのサンプルが入るようにする。
        // 場合によってアップ・ダウンミックスするかも。
        //
        // Channels : 5なら、
        // [0,1,2,3,4][0,1,2,3,4]...のようにバッファのサンプル構成を入れる。
        self.buffer_sender.write_with(requiring_samples, move |buffer| {
            // 1. inputをforにして、frame_iを増加する。
            // 2. frame_iがframe_countより同じか大きければ、抜ける。
            let buffer_len = buffer.len();
            let frame_count = buffer_len / channels;
            if frame_count <= 0 {
                return;
            }
            let mut frame_i = 0usize;

            match channel_buffers {
                EDrainedChannelBuffers::Mono { channel } => {
                    for sample in channel {
                        let start_i = frame_i * channels;
                        let end_i = start_i + channels;
                        remix_mono_to(sample, channels, &mut buffer[start_i..end_i]);

                        frame_i += 1;
                        if frame_i >= frame_count {
                            break;
                        }
                    }
                }
                EDrainedChannelBuffers::Stereo { ch_left, ch_right } => {
                    debug_assert_eq!(ch_left.len(), ch_right.len());

                    for (l_sample, r_sample) in ch_left.into_iter().zip_eq(ch_right) {
                        let start_i = frame_i * channels;
                        let end_i = start_i + channels;
                        remix_stereo_to(l_sample, r_sample, channels, &mut buffer[start_i..end_i]);

                        frame_i += 1;
                        if frame_i >= frame_count {
                            break;
                        }
                    }
                }
            }
        });
    }
}

/// `sample`のモノ音源を任意でミックスして`buffer`に入れる。
fn remix_mono_to(sample: UniformedSample, channels: usize, buffer: &mut [f32]) {
    debug_assert!(channels > 0);
    debug_assert!(buffer.len() >= channels);

    let sample = sample.to_f64_clamped() as f32;
    for i in 0..channels {
        buffer[i] = sample;
    }
}

/// `left`, `right`のステレオ音源を任意でミックスして`buffer`に入れる。
fn remix_stereo_to(left: UniformedSample, right: UniformedSample, channels: usize, buffer: &mut [f32]) {
    debug_assert!(channels > 0);
    debug_assert!(buffer.len() >= channels);

    // 24-12-12 ダウンミックスとそのままだけ。
    // @todo 多重チャンネルも対応しよう。
    match channels {
        1 => {
            // ダウンミックスしよう。
            //
            // https://www.sonible.com/blog/stereo-to-mono/
            // でも足して割り算するだけだとフェーズ相殺によるコムフィルタになってしまうけど、
            // しょうがないか。。。
            let downmixed = (left.to_f64_clamped() + right.to_f64_clamped()) * 0.5;
            buffer[0] = downmixed as f32;
        }
        2 => {
            buffer[0] = left.to_f64_clamped() as f32;
            buffer[1] = right.to_f64_clamped() as f32;
        }
        _ => unreachable!(),
    }
}

type AudioDeviceProxyPtr = Arc<Mutex<AudioDeviceProxy>>;
pub type AudioDeviceProxyWeakPtr = Weak<Mutex<AudioDeviceProxy>>;

// ----------------------------------------------------------------------------
// EDrainedChannelBuffers
// ----------------------------------------------------------------------------

/// [`OutputDeviceProcessData`]の内部で送信用の各チャンネルのバッファをまとめたもの。
#[derive(Debug)]
pub enum EDrainedChannelBuffers {
    Mono {
        channel: Vec<UniformedSample>,
    },
    Stereo {
        ch_left: Vec<UniformedSample>,
        ch_right: Vec<UniformedSample>,
    },
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
