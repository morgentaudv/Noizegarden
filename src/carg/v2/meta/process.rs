
/// ノードの依存システムのカテゴリビットフラグ
pub mod process_category {
    /// ノーマル
    pub const NORMAL: u64 = 0;

    /// デバイスアウトプットのノードだけ使用。
    /// 他のノードで使うのは禁止。
    pub const BUS_MASTER_OUTPUT: u64 = 1 << 63;
}

/// [`process_category`]のフラグ制御のための補助タイプ
pub type EProcessCategoryFlag = u64;

/// [`TProcess`]の処理の設定やメタ情報を記載するためのTrait。
pub trait TProcessCategory {
    /// ノードの処理をどの順に行うかを決める。
    ///
    /// 現在（24-12-12）は複数のフラグを入れて複数の処理グループで選択的に行わせるのは禁止。
    fn get_process_category() -> EProcessCategoryFlag {
        process_category::NORMAL
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
