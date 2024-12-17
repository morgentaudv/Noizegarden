use serde::{Deserialize, Serialize};

/// フレームTickのモード
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ETimeTickMode {
    #[serde(rename = "offline")]
    Offline,
    #[serde(rename = "realtime")]
    Realtime,
}

/// メタTrait。
/// 各ノードの[`ETimeTickMode`]のサポート可否を指定する。
pub trait TTimeTickCategory {
    /// オフライン処理（バッチ処理）ができるか？
    fn can_support_offline() -> bool {
        false
    }

    /// リアルタイムの処理ができるか？
    fn can_support_realtime() -> bool {
        false
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
