use crate::file::reader::{FileReader, FileReaderSetting};
use crate::file::writer::FileWriter;
use crate::file::{EFileAccessSetting, FileController, FileControllerPtr, FileControllerWeakPtr};
use std::ops::Deref;
use std::sync::Arc;

/// システム外部からファイルの操作を行うためのハンドル。
/// RAIIでドロップしたらファイル制御の破棄通知がいく。
pub struct FileHandle {
    /// 接近用
    v: FileControllerWeakPtr,
    /// 識別用
    id: usize,
    /// [`FileController`]がもっているハンドルかそうではないか。
    /// trueなら、Dropする時にカウント処理を行わない。
    pub(super) is_internal: bool,
}

impl PartialEq for FileHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for FileHandle {}

impl Clone for FileHandle {
    fn clone(&self) -> Self {
        let item = Self {
            v: self.v.clone(),
            id: self.id,
            is_internal: self.is_internal,
        };

        if !item.is_internal {
            match item.v.upgrade() {
                None => {}
                Some(v) => {
                    v.lock().unwrap().handle_count += 1;
                }
            }
        }

        item
    }
}

impl Drop for FileHandle {
    fn drop(&mut self) {
        // 内部用のハンドルなら何もしない。
        if self.is_internal { return; }

        // 落とす。
        self.v.upgrade().unwrap().lock().unwrap().handle_count -= 1;
    }
}

impl FileHandle {
    /// アイテムが持つ内部ハンドルを作る。
    pub(super) fn new_internal_handle(item: FileControllerPtr) -> Self {
        // ここでアイテムのアドレスを無理やり取得して、それをIDとして扱う。
        // そして内部用ハンドルを作る。
        let id = {
            let guard = item.lock().unwrap();
            let new_item = guard.deref();
            let address_as_id = new_item as *const FileController as usize;
            address_as_id
        };

        let v = Arc::downgrade(&item);
        Self {
            v,
            id,
            is_internal: true,
        }
    }

    /// 書き込み可能状態なら、何かを書き込めるものを用意する。
    pub fn try_write(&self) -> Option<Box<FileWriter>> {
        let is_writable = match self.v.upgrade() {
            None => false,
            Some(v) => {
                let v = v.lock().unwrap();
                match v.setting {
                    EFileAccessSetting::Write { .. } => true,
                    _ => false
                }
            }
        };

        if !is_writable {
            None
        }
        else {
            // 25-01-01 BufWriterを使ってみる。
            // 既存ロジックのコードをほぼそのまま持ってくる。
            let v = self.v.upgrade().unwrap();
            assert!(v.lock().unwrap().internal.is_some());

            let item = FileWriter::new(v);
            Some(item)
        }
    }

    /// 読み込み可能状態なら、ファイルから読み込めるようにするためのものを用意する。
    pub fn try_read(&self, setting: FileReaderSetting) -> Option<Box<FileReader>> {
        let is_readable = match self.v.upgrade() {
            None => false,
            Some(v) => {
                let v = v.lock().unwrap();
                match v.setting {
                    EFileAccessSetting::Read { .. } => true,
                    _ => false
                }
            }
        };

        if !is_readable {
            None
        }
        else {
            let v = self.v.upgrade().unwrap();
            assert!(v.lock().unwrap().internal.is_some());

            let item = FileReader::new(v, setting);
            Some(item)
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
