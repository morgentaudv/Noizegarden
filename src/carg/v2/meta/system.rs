use crate::device::AudioDeviceSetting;
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemSetting {
    /// [`AudioDevice`]の設定
    pub audio_device: Option<AudioDeviceSetting>,
}

impl SystemSetting {
    /// シリアライズされた情報から読み込む。
    pub fn from_serde_value(value: Value) -> anyhow::Result<Self> {
        let setting: Self = serde_json::from_value(value)?;
        Ok(setting)
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
