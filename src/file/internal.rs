use std::fs;
use crate::file::EFileAccessSetting;

pub(super) enum EInternalData {
    Write {
        /// ファイルのDescriptorなどを保持。
        file: std::fs::File,
    },
    Read {
        /// ファイルのDescriptorなどを保持。
        file: std::fs::File,
    }
}

impl EInternalData {
    pub fn new(setting: &EFileAccessSetting) -> Self {
        match setting {
            EFileAccessSetting::Write { path } => {
                // 25-01-02 ハンドルを作る。今はそれだけ。
                Self::Write {
                    file: fs::File::create(path).expect("Could not create a file.")
                }
            }
            EFileAccessSetting::Read { path } => {
                // ファイルを読みこんで、設定からバッファを読み取るなどをする。
                let file = fs::File::open(&path).expect(&format!("Could not find {}.", &path));

                Self::Read {
                    file,
                }
            }
        }
    }
}


// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
