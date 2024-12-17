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

/// Implement [`TTimeTickCategory`] for given `t` type,
/// `offline` value must be return value of `can_support_offline`, and `realtime` must be.
#[macro_export]
macro_rules! nz_define_time_tick_for {
    ($t: ty, $offline: expr, $realtime: expr) => {
        impl TTimeTickCategory for $t {
            fn can_support_offline() -> bool {
                $offline
            }

            fn can_support_realtime() -> bool {
                $realtime
            }
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
