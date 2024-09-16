use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum EFloatCommonPin {
    #[serde(rename = "constant")]
    Constant(f64),
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
