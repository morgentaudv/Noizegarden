use crate::wave::sample::UniformedSample;
use itertools::Itertools;
use miniaudio::{DeviceType, FramesMut, RingBufferRecv, RingBufferSend};
use std::sync::{mpsc, Arc, Mutex, OnceLock, Weak};
use serde::{Deserialize, Serialize};

/// 24-12-10
/// mutにしているのは、[`AudioDevice::cleanup()`]で値をTakeするため。
/// 他に良い方法があればそれにしてmutをなくしたい。
///
/// 24-12-23
/// Takeしなくても、中にInternal変数を持たせてOptionにすることでTakeせずに解放できるようになった。
/// なので`mut`しなくても更新できるようになった。
static AUDIO_DEVICE: OnceLock<Arc<Mutex<AudioDevice>>> = OnceLock::new();

/// コールバックから取得する必要があるので、[`AudioDevice`]には入れない。
/// リングバッファのレシーバー
static BUFFER_RECEIVER: OnceLock<Mutex<Option<RingBufferRecv<f32>>>> = OnceLock::new();

/// デバイスの処理関数でデバイスに接近するためのItem。
/// デバイスの初期化時に登録される。
/// WeakPtrなので解放はしなくてもいいかもしれない。
static PROXY_ACCESSOR: OnceLock<AudioDeviceProxyWeakPtr> = OnceLock::new();

/// 依存システム全体からの処理の結果
#[derive(Debug)]
pub enum ESystemProcessResult {
    /// 何も起きていない。次フレームでも処理が続行できる。
    Nothing,
    /// システムが停止したので、これから処理をしてはいけない。
    SystemStopped,
}

#[derive(Debug)]
enum EAudioDeviceState {
    NotStarted,
    Started,
    Stopped,
}

/// デバイスシステムの処理すべく外部から送られてくる通知やメッセージ。
pub enum EAudioDeviceMessage {
    Stop,
    /// オーディオのStarvationが起きている。
    StarvationNotified(usize),
    /// オーディオ内部処理で音を流すためのバッファのサンプル数を含む。
    LastProcessedLength(usize),
    /// オーディオ処理のバッファに`usize`分のサンプルを送信した。
    SendSamplesToBuffer(usize),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioDeviceSetting {
    /// 初期チャンネル数
    pub channels: usize,
    /// 初期サンプルレート
    pub sample_rate: usize,
}

/// [`AudioDevice`]を生成するための初期設定のための構造体。
#[derive(Debug, Clone)]
pub struct AudioDeviceConfig {
    /// 初期チャンネル数
    channels: usize,
    /// 初期サンプルレート
    sample_rate: usize,
    /// リングバッファの1フレーム処理推定時間 (ms単位)
    frame_ideal_milliseconds: std::time::Duration,
}

impl AudioDeviceConfig {
    /// デフォルトインスタンスの生成。
    pub fn new() -> Self {
        Self {
            channels: 0,
            sample_rate: 0,
            frame_ideal_milliseconds: std::time::Duration::from_millis(5),
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

/// [`AUdioDevice`]の内部更新情報をまとめた構造体。
#[derive(Debug)]
struct AudioDeviceStateInfo {
    /// 内部制御状態
    state: EAudioDeviceState,
    /// 現在設定したリングバッファのサブバッファのサンプル数。
    /// 24-12-15 後でデバイス設定の変更からリセットのロジックが組まれたらこれも更新になるかもしれない。
    sub_buffer_len: usize,
    /// 現在処理が残っていると思われるバッファのサンプルのカウント。
    /// 更新タイミングは詩システムのTick時点。
    remained_samples_count: usize,
    /// 前フレームの処理で出力で読み込んだサンプル数。
    /// 更新タイミングは詩システムのTick時点。
    prev_processed_samples_count: usize,
    ///
    total_required_samples: usize,
    /// 前フレームでStarvationが起きているか？
    is_starvation: bool,
}

pub struct AudioDeviceInternal {
    /// ローレベルのデバイス
    low_device: miniaudio::Device,
    /// プロキシの親元。ほかのところでは全部Weakタイプで共有する。
    original_proxy: Option<AudioDeviceProxyPtr>,
    /// [`AudioDevice::process`]から取得して特定の処理を行うためのもの。
    rx: Option<mpsc::Receiver<EAudioDeviceMessage>>,
    /// 更新情報
    info: AudioDeviceStateInfo,
    /// 初期設定
    #[allow(dead_code)]
    initial_config: AudioDeviceConfig,
}

impl AudioDeviceInternal {
    fn new(config: AudioDeviceConfig) -> Self {
        assert!(config.channels > 0);
        assert!(config.sample_rate > 0);

        let mut low_device_config = miniaudio::DeviceConfig::new(DeviceType::Playback);
        low_device_config.playback_mut().set_format(miniaudio::Format::F32);
        low_device_config.playback_mut().set_channels(config.channels as u32);
        low_device_config.set_sample_rate(config.sample_rate as u32);
        low_device_config.set_data_callback(AudioDevice::on_update_device_callback);
        low_device_config.set_stop_callback(AudioDevice::on_stop_device_callback);

        let low_device = miniaudio::Device::new(None, &low_device_config).expect("failed to create audio device");
        Self {
            low_device,
            original_proxy: None, // これはあとで初期化する。
            rx: None,
            info: AudioDeviceStateInfo {
                state: EAudioDeviceState::NotStarted,
                sub_buffer_len: AudioDevice::calculate_ring_sub_buffer_length(&config),
                remained_samples_count: 0,
                prev_processed_samples_count: 0,
                total_required_samples: 0,
                is_starvation: true, // 最初はStarvationありにして最大限のサンプル数を取得させる。
            },
            initial_config: config.clone(),
        }
    }

    /// 今デバイスに設定しているチャンネルの数を返す。
    /// もしデバイスが無効になっているのであれば、`0`を返す。
    pub fn get_channels(&self) -> usize {
        self.low_device.playback().channels() as usize
    }

    pub fn pre_process(&mut self, _frame_time: f64) {
        match self.info.state {
            EAudioDeviceState::NotStarted => {
                self.low_device.start().expect("Failed to start audio device");
                self.info.state = EAudioDeviceState::Started;
            }
            _ => {}
        }
    }

    /// Tick関数。
    pub fn post_process(&mut self, _frame_time: f64) -> ESystemProcessResult {
        match self.info.state {
            EAudioDeviceState::Started => {
                self.process_started();
            }
            _ => {}
        }

        ESystemProcessResult::Nothing
    }

    /// デバイスの状態が[`EAudioDeviceState::Started`]な時の専用処理関数。
    fn process_started(&mut self) {
        // 関連変数を初期化する。
        self.info.prev_processed_samples_count = 0;

        let mut is_starvation = false;
        let mut last_processed_samples_length = 0usize;
        let mut last_send_buffer_length = 0usize;
        let mut required_length = 0usize;

        // メッセージの処理を行う。
        let rx = self.rx.as_ref().unwrap();
        loop {
            let message = match rx.try_recv() {
                Ok(v) => v,
                Err(_) => {
                    break;
                }
            };

            match message {
                EAudioDeviceMessage::Stop => {
                    self.info.state = EAudioDeviceState::Stopped;
                }
                EAudioDeviceMessage::StarvationNotified(required_count) => {
                    // もしこのフレームで1回でもStarvationが起きたのであれば、
                    // Starvation上での処理を優先する。
                    is_starvation = true;
                    required_length += required_count;
                }
                EAudioDeviceMessage::LastProcessedLength(samples_count) => {
                    last_processed_samples_length += samples_count;
                    required_length += samples_count;
                }
                EAudioDeviceMessage::SendSamplesToBuffer(samples_count) => {
                    last_send_buffer_length += samples_count;
                }
            }
        }

        // バッファ関連の情報更新
        self.info.remained_samples_count += last_send_buffer_length;
        if self.info.remained_samples_count >= last_processed_samples_length {
            self.info.remained_samples_count -= last_processed_samples_length;
        } else {
            self.info.remained_samples_count = 0;
        }
        self.info.prev_processed_samples_count = last_processed_samples_length;
        self.info.is_starvation = is_starvation;
        self.info.total_required_samples += required_length;

        if self.info.is_starvation {
            println!("Starved!");
        }
    }
}

pub struct AudioDevice {
    v: Option<AudioDeviceInternal>,
}

impl AudioDevice {
    /// リングバッファの最低限のサンプル数。
    /// フレーム理想処理時間からの換算のサンプル数がこれ未満でも、これを適用する。
    const BUFFER_MINIMUM_SAMPLES: usize = 1024;

    /// サブバッファのサンプル数の数を計算する。
    fn calculate_ring_sub_buffer_length(config: &AudioDeviceConfig) -> usize {
        let ideal_seconds = config.frame_ideal_milliseconds.as_secs_f64();
        let raw_required_samples = (config.sample_rate as f64 * config.channels as f64 * ideal_seconds).ceil() as usize;

        let required_samples = raw_required_samples.next_power_of_two();
        required_samples.max(Self::BUFFER_MINIMUM_SAMPLES)
    }

    /// システムを初期化する。
    /// すべての処理（レンダリング）が始まる前に処理すべき。
    pub fn initialize(config: AudioDeviceConfig) -> AudioDeviceProxyWeakPtr {
        // RingBufferの登録。
        // RingBufferのタイプはf32にして、あとで受け取る側でいい変換して送る。
        //
        // ただし生成したままではちゃんと扱えないので、sendだけはArc<Mutex<>>にはさむ。
        let sub_buffer_len = Self::calculate_ring_sub_buffer_length(&config);
        let (send, recv) =
            miniaudio::ring_buffer::<f32>(sub_buffer_len, 16).expect("Failed to create audio ring buffer.");
        let _result = BUFFER_RECEIVER.set(Mutex::new(Some(recv)));

        // メッセージチャンネルの生成と登録。
        let (tx, rx) = mpsc::channel();

        // @todo 24-12-10 ここら辺のコード、結構危なっかしいのであとでちゃんとしたものに書き換えしたい。
        // こっからProxyを作って、weakを渡してから
        let original_proxy = {
            assert!(AUDIO_DEVICE.get().is_none());

            // デバイスの初期化
            let _result = AUDIO_DEVICE.set(Arc::new(Mutex::new(Self::new(config))));
            let device = AUDIO_DEVICE.get().unwrap();
            let weak_device = Arc::downgrade(&device);

            let original_proxy = AudioDeviceProxy::new(weak_device, send, tx);
            original_proxy
        };

        // Proxyの登録。
        let weak_proxy = Arc::downgrade(&original_proxy);
        {
            // Mutexがおそらく内部Internal Mutabilityを実装しているかと。
            let instance = AUDIO_DEVICE.get().expect("AudioDevice instance must be valid");
            let mut accessor = instance.lock().unwrap();
            debug_assert!(accessor.v.is_some());

            // 24-12-23 内部に接近する。
            let v = accessor.v.as_mut().unwrap();
            v.original_proxy = Some(original_proxy);
            v.rx = Some(rx);
        }

        // Proxyを返す。本体は絶対返さない。
        assert!(AUDIO_DEVICE.get().is_some());

        // 24-12-15 登録。
        let _result = PROXY_ACCESSOR.set(weak_proxy.clone());
        weak_proxy
    }

    /// システムの対応。
    pub fn get_proxy() -> Option<AudioDeviceProxyWeakPtr> {
        // これは大丈夫か。。。。
        match PROXY_ACCESSOR.get() {
            None => None,
            Some(v) => Some(v.clone()),
        }
    }

    pub fn pre_process(_frame_time: f64) {
        assert!(AUDIO_DEVICE.get().is_some());

        {
            let instance = AUDIO_DEVICE.get().expect("AudioDevice instance must be valid");
            let mut accessor = instance.lock().unwrap();
            debug_assert!(accessor.v.is_some());
            let v = accessor.v.as_mut().unwrap();

            v.pre_process(_frame_time);
        }
    }

    /// Tick関数。
    pub fn post_process(_frame_time: f64) -> ESystemProcessResult {
        assert!(AUDIO_DEVICE.get().is_some());

        {
            let instance = AUDIO_DEVICE.get().expect("AudioDevice instance must be valid");
            let mut accessor = instance.lock().unwrap();
            debug_assert!(accessor.v.is_some());
            let v = accessor.v.as_mut().unwrap();
            v.post_process(_frame_time)
        }
    }

    /// システムを解放する。
    /// すべての関連処理が終わった後に解放すべき。
    pub fn cleanup() {
        assert!(AUDIO_DEVICE.get().is_some());

        // 12-11-xx ここでdropするので、もう1回解放してはいけない。
        // 12-12-23 Optionおdropすればいいだけ。
        if let Some(device) = AUDIO_DEVICE.get() {
            let mut device = device.lock().unwrap();
            device.v = None;
        }

        // Receiverも解放する。
        if let Some(rv) = BUFFER_RECEIVER.get() {
            let mut rv = rv.lock().unwrap();
            *rv = None;
        }
    }

    fn new(config: AudioDeviceConfig) -> Self {
        assert!(config.channels > 0);
        assert!(config.sample_rate > 0);

        Self {
            v: Some(AudioDeviceInternal::new(config)),
        }
    }

    /// 今デバイスに設定しているチャンネルの数を返す。
    /// もしデバイスが無効になっているのであれば、`0`を返す。
    pub fn get_channels(&self) -> usize {
        self.v.as_ref().unwrap().get_channels()
    }

    fn on_update_device_callback(device: &miniaudio::RawDevice, output: &mut FramesMut, _input: &miniaudio::Frames) {
        const ATTEMPTS_COUNT: usize = 8;
        debug_assert!(BUFFER_RECEIVER.get().is_some());

        let mut read_count = 0;
        let mut attempts = 0;
        let mut required_output_length = 0usize;

        match device.playback().format() {
            miniaudio::Format::S16 => {
                let outputs = output.as_samples_mut::<i16>();
                required_output_length = outputs.len();

                // f32は[-1, 1]までに。
                let mut raw_samples = vec![];
                raw_samples.resize(required_output_length, 0.0f32);

                // できるだけ読み切る。
                while read_count < required_output_length && attempts < ATTEMPTS_COUNT {
                    read_count += BUFFER_RECEIVER
                        .get()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .read(&mut raw_samples[read_count..]);
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
                required_output_length = outputs.len();

                // Here we try reading at most 8 sub buffers to attempt to read enough outputs to
                // fill the playback output buffer. We don't allow infinite attempts because we can't be
                // sure how long that would take.
                while read_count < required_output_length && attempts < ATTEMPTS_COUNT {
                    read_count += BUFFER_RECEIVER
                        .get()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .read(&mut outputs[read_count..]);
                    attempts += 1;
                }

                // If we're starved, just repeat the last sample on all channels:
                (&mut outputs[read_count..]).iter_mut().for_each(|s| *s = 0.0);
            }
            _ => unreachable!(),
        }

        // もしStarvationが発生したら、次のバッファ取得値は最大限にする。
        if required_output_length > 0 {
            let proxy = PROXY_ACCESSOR.get().unwrap().upgrade().unwrap();
            let accessor = proxy.lock().unwrap();

            if read_count < required_output_length {
                accessor
                    .tx
                    .send(EAudioDeviceMessage::StarvationNotified(required_output_length))
                    .expect("Message could not send.");
            } else {
                accessor
                    .tx
                    .send(EAudioDeviceMessage::LastProcessedLength(required_output_length))
                    .expect("Message could not send.");
            }
        }
    }

    fn on_stop_device_callback(_device: &miniaudio::RawDevice) {}
}

unsafe impl Sync for AudioDevice {}

/// レンダリングアイテムからデバイスに接近するためのプロキシ。
pub struct AudioDeviceProxy {
    /// デバイスに接近するためのもの。
    device: Weak<Mutex<AudioDevice>>,
    /// 最終オーディオレンダリングに使うためのリングバッファ。
    buffer_sender: RingBufferSend<f32>,
    /// Multi-producerなので、おそらく内部でThread-safeなはず。
    /// [`AudioDevice`]の処理までに特定の動作を送るため。
    tx: mpsc::Sender<EAudioDeviceMessage>,
}

impl AudioDeviceProxy {
    /// 親元となるProxyのマルチスレッド版のインスタンスを生成する。
    fn new(
        device: Weak<Mutex<AudioDevice>>,
        buffer_sender: RingBufferSend<f32>,
        tx: mpsc::Sender<EAudioDeviceMessage>,
    ) -> AudioDeviceProxyPtr {
        let instance = Self {
            device,
            buffer_sender,
            tx,
        };
        Arc::new(Mutex::new(instance))
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
    pub fn send_sample_buffer_with<F>(&self, f: F) -> usize
    where
        F: FnOnce(/*frame_count:*/ usize) -> EDrainedChannelBuffers,
    {
        let channels = self.get_channels();
        if channels == 0 {
            return 0;
        }

        self.buffer_sender.write_with(1024, move |buffer| {
            let buffer_len = buffer.len();
            let frame_count = buffer_len / channels;
            if frame_count <= 0 {
                return;
            }

            let channel_buffers = f(frame_count);

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
        })
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
