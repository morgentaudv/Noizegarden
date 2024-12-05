use serde::{Deserialize, Serialize};

pub mod output_file;
pub mod output_log;
pub mod output_device;

/// ファイルとして出力するときのノード。
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum EOutputFileFormat {
    #[serde(rename = "wav_lpcm16")]
    WavLPCM16 { sample_rate: u64 },
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
