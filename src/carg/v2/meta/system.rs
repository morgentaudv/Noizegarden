use crate::device::{AudioDevice, AudioDeviceConfig, AudioDeviceProxyWeakPtr, AudioDeviceSetting};
use crate::file::{FileIO, FileIOProxy, FileIOProxyWeakPtr, FileIOSetting};
use crate::resample::{ResampleSystem, ResampleSystemConfig, ResampleSystemProxyWeakPtr};
use num_traits::Zero;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// ノードの依存システムのカテゴリのビットフラグ
pub mod system_category {
    /// 何も依存しない。
    pub const NONE: u32 = 0;

    /// オーディオデバイスに依存
    pub const AUDIO_DEVICE: u32 = 1 << 0;

    /// リサンプリング処理に必要なシステム
    pub const RESAMPLE_SYSTEM: u32 = 1 << 1;

    /// ファイル読み込み、書き込み、ストリーミング制御システム
    /// @todo 実装すること。
    pub const FILE_IO_SYSTEM: u32 = 1 << 2;

    /// @todo Emitter系ターゲットにして実装。
    #[allow(dead_code)]
    pub const REALTIME_TRIGGER_SYSTEM: u32 = 1 << 3;
}

/// [`system_category`]のフラグ制御の補助タイプ
pub type ESystemCategoryFlag = u32;

/// メタTrait。
pub trait TSystemCategory {
    /// 音処理ノードの依存システムを複数のフラグにして返す。
    fn get_dependent_system_categories() -> ESystemCategoryFlag {
        system_category::NONE
    }
}

/// シリアライズできるシステムの設定コンテナ
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemSetting {
    /// [`AudioDevice`]の設定
    pub audio_device: Option<AudioDeviceSetting>,
    /// [`FileIO`]の設定
    pub file_io: Option<FileIOSetting>,
}

impl SystemSetting {
    /// シリアライズされた情報から読み込む。
    pub fn from_serde_value(value: Value) -> anyhow::Result<Self> {
        let setting: Self = serde_json::from_value(value)?;
        Ok(setting)
    }
}

/// [`initialize_systems`]関数の結果。
/// 初期化したシステムのアクセスアイテムなどが入っている。
#[derive(Debug, Default, Clone)]
pub struct InitializeSystemAccessor {
    /// [`AudioDevice`]システムに接近できるアクセサー
    pub audio_device: Option<AudioDeviceProxyWeakPtr>,
    /// [`ResampleSystem`]システムに接近できるアクセサー
    pub resample_system: Option<ResampleSystemProxyWeakPtr>,
    /// [`FileIO`]システムに接近できるアクセサー
    pub file_io: Option<FileIOProxyWeakPtr>,
}

impl InitializeSystemAccessor {
    /// [`FileIO`]に接近する。
    pub fn access_file_io_fn<F>(&self, f: F) where F: FnOnce(/*system: */ &mut FileIOProxy) {
        // うわ。。あんまりだ。
        if let Some(v) = &self.file_io {
            let v = v.upgrade().expect("Must be successful");
            let mut v = v.lock();
            let mut v = v.as_deref_mut().unwrap();
            f(&mut v);
        }
    }
}

/// `flags`から関連システムを初期化する。
/// 一回きりで実行すべき。
pub fn initialize_systems(flags: ESystemCategoryFlag, system_setting: &SystemSetting) -> InitializeSystemAccessor {
    let mut result = InitializeSystemAccessor::default();

    // FileIOSystemの初期化
    if !(flags & system_category::FILE_IO_SYSTEM).is_zero() {
        let setting = system_setting.file_io.as_ref().expect("FileIOSetting not set.");
        result.file_io = Some(FileIO::initialize(setting.clone()));
    }

    // AudioDeviceの初期化
    if !(flags & system_category::AUDIO_DEVICE).is_zero() {
        let setting = system_setting.audio_device.as_ref().expect("AudioDeviceSetting not set");
        assert!(setting.channels > 0);

        let mut config = AudioDeviceConfig::new();
        config.set_channels(setting.channels).set_sample_rate(setting.sample_rate);
        result.audio_device = Some(AudioDevice::initialize(config));
    }

    // ResampleSystemの初期化
    if (!flags & system_category::RESAMPLE_SYSTEM).is_zero() {
        let config = ResampleSystemConfig::new();
        result.resample_system = Some(ResampleSystem::initialize(config));
    }

    result
}

/// グラフ処理前のシステムの前処理
pub fn preprocess_systems(flags: ESystemCategoryFlag, prev_to_now_time: f64) {
    if !(flags & system_category::AUDIO_DEVICE).is_zero() {
        AudioDevice::pre_process(prev_to_now_time);
    }
}

/// グラフ処理後のシステムの後処理
pub fn postprocess_systems(flags: ESystemCategoryFlag, prev_to_now_time: f64) {
    if !(flags & system_category::AUDIO_DEVICE).is_zero() {
        AudioDevice::post_process(prev_to_now_time);
    }

    // FileIOSystemの処理
    if !(flags & system_category::FILE_IO_SYSTEM).is_zero() {
        FileIO::post_process(prev_to_now_time);
    }
}

/// 依存システムの解放
pub fn cleanup_systems(flags: ESystemCategoryFlag) {
    // AudioDeviceの解放
    if !(flags & system_category::AUDIO_DEVICE).is_zero() {
        AudioDevice::cleanup();
    }

    // ResampleSystemの解放
    if (!flags & system_category::RESAMPLE_SYSTEM).is_zero() {
        ResampleSystem::cleanup();
    }

    // FileIOSystemの解放
    if !(flags & system_category::FILE_IO_SYSTEM).is_zero() {
        FileIO::cleanup();
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
