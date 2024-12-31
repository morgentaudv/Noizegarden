use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::carg::v2::meta::tick::ETimeTickMode;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Setting {
    pub time_tick_mode: ETimeTickMode,
    /// 更新時の推奨されるサンプル数。
    /// たとえば48kHzだと約21ms弱ぐらいになる。
    /// この値は必ず2のべき乗数でなければならない。
    pub sample_count_frame: usize,
}

impl Setting {
    pub fn from_serde_value(value: Value) -> anyhow::Result<Self> {
        let setting: Setting = serde_json::from_value(value)?;
        if !setting.sample_count_frame.is_power_of_two() {
            return Err(anyhow::anyhow!(
            "Given `sample_count_frame` is not power of two. (256, 512, 1024...)"
            ));
        }

        Ok(setting)
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
