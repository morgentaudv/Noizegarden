/// ノードの依存システムのカテゴリのビットフラグ
pub mod system_category {
    /// 何も依存しない。
    pub const NONE: u32 = 0;

    /// オーディオデバイスに依存
    pub const AUDIO_DEVICE: u32 = 1 << 0;

    /// リサンプリング処理に必要なシステム
    pub const RESAMPLE_SYSTEM: u32 = 1 << 1;
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

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
