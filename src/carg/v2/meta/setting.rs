use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::carg::v2::meta::tick::ETimeTickMode;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Setting {
    pub time_tick_mode: ETimeTickMode,
    /// [`ETimeTickMode::Realtime`]処理モードで、
    /// フレーム時間が多くなっても指定した時間より多くのサンプルを処理しないようにする。
    pub process_limit_time: f64,
}

impl Setting {
    pub fn from_serde_value(value: Value) -> anyhow::Result<Self> {
        let setting: Setting = serde_json::from_value(value)?;
        if setting.process_limit_time <= 0.0 {
            return Err(anyhow::anyhow!(
            "Given `process_limit_time` must be positive second value"
            ));
        }

        Ok(setting)
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
