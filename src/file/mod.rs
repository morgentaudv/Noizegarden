mod handle;
mod writer;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::{fs, io};
use std::io::Write;
use std::marker::PhantomData;
use crate::file::handle::FileHandle;

/// 24-12-31
/// ファイルIO制御のシステム
static SYSTEM: OnceLock<Arc<Mutex<FileIO>>> = OnceLock::new();

/// システムアクセス用。
/// デバイスの初期化時に登録される。
/// WeakPtrなので解放はしなくてもいいかもしれない。
static PROXY_ACCESSOR: OnceLock<FileIOProxyWeakPtr> = OnceLock::new();

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileIOSetting {

}

/// ファイルIOの処理構造体
pub struct FileIO {
    v: Option<FileIOInternal>,
}

impl FileIO {
    pub fn initialize(setting: FileIOSetting) -> FileIOProxyWeakPtr {
        let original_proxy = {
            assert!(SYSTEM.get().is_none());

            let _result = SYSTEM.set(Arc::new(Mutex::new(Self::new(setting))));
            let system = SYSTEM.get().unwrap();
            let weak_system = Arc::downgrade(&system);

            let original_proxy = FileIOProxy::new(weak_system);
            original_proxy
        };

        // Proxyの登録
        let weak_proxy = Arc::downgrade(&original_proxy);
        {
            // Mutexがおそらく内部Internal Mutabilityを実装しているかと。
            let instance = SYSTEM.get().expect("AudioDevice instance must be valid");
            let mut accessor = instance.lock().unwrap();
            debug_assert!(accessor.v.is_some());

            // 24-12-23 内部に接近する。
            let v = accessor.v.as_mut().unwrap();
            v.original_proxy = Some(original_proxy);
        }

        // Proxyを返す。本体は絶対かえさない。
        assert!(SYSTEM.get().is_some());
        let _result = PROXY_ACCESSOR.set(weak_proxy.clone());
        weak_proxy
    }

    fn new(setting: FileIOSetting) -> FileIO {
        Self {
            v: Some(FileIOInternal::new(setting)),
        }
    }

    /// システムの対応。
    pub fn get_proxy() -> Option<FileIOProxyWeakPtr> {
        // これは大丈夫か。。。。
        match PROXY_ACCESSOR.get() {
            None => None,
            Some(v) => Some(v.clone()),
        }
    }

    /// システムを解放する。
    /// すべての関連処理が終わった後に解放すべき。
    pub fn cleanup() {
        assert!(SYSTEM.get().is_some());

        if let Some(system) = SYSTEM.get() {
            let mut system = system.lock().unwrap();
            system.v = None;
        }
    }
}

// ----------------------------------------------------------------------------
// Proxy
// ----------------------------------------------------------------------------

pub struct FileIOProxy {
    /// システムに接近するための変数。
    system: Weak<Mutex<FileIO>>,
}

impl FileIOProxy {
    fn new(
        system: Weak<Mutex<FileIO>>,
    ) -> FileIOProxyPtr {
        let instance = Self {
            system,
        };
        Arc::new(Mutex::new(instance))
    }

    pub fn create_handle(&self, setting: EFileAccessSetting) -> FileHandle {
        let system = self.system.upgrade().unwrap();
        let mut system = system.lock().unwrap();
        let system = system.v.as_mut().unwrap();

        system.create_handle(setting)
    }
}

type FileIOProxyPtr = Arc<Mutex<FileIOProxy>>;
pub type FileIOProxyWeakPtr = Weak<Mutex<FileIOProxy>>;

/// ファイル展開の設定。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EFileAccessSetting {
    /// テキストで書き込み専用
    Write {
        path: String,
    }
}

// ----------------------------------------------------------------------------
// Controller
// ----------------------------------------------------------------------------

/// 内部ファイル接近の制御コントローラー
///
/// 内部にハンドルを持っていて、他のところから接近したい時にもっているハンドルを返すようにしたい。
struct FileController {
    #[allow(dead_code)]
    setting: EFileAccessSetting,
    /// 内部保持用のハンドル。増殖用。
    handle: Option<FileHandle>,
    /// 外部からもっているハンドルのカウント
    /// 自分が持っているハンドルがカウントに含まない。
    ///
    /// もし0なら、このコントロールアイテムをシステム側から消すことが可能。
    handle_count: isize,
    /// ファイルの制御状態
    state: FileControllerState,
    /// 読み込み・書き込みのファイル先
    file: Option<fs::File>,
}

/// ファイルの読み込み・書き込み状態
#[derive(Debug)]
enum FileControllerState {
    Idle,
    Flying,
}

type FileControllerPtr = Arc<Mutex<FileController>>;
type FileControllerWeakPtr = Weak<Mutex<FileController>>;

impl FileController {
    /// このファイルに接近するためのハンドルを返す。
    fn create_handle(&mut self) -> FileHandle {
        assert!(self.handle.is_some());

        // カウント１増加し、返すハンドルは外部使用になるのでフラグを更新して処理分岐する。
        self.handle_count += 1;
        // ここではハンドルカウントを増やさない。
        let mut handle = self.handle.as_ref().unwrap().clone();
        handle.is_internal = false;
        handle
    }

    /// 新しく作る。
    fn new(setting: EFileAccessSetting) -> FileControllerPtr {
        let item = Self {
            setting,
            handle: None,
            handle_count: 0,
            state: FileControllerState::Idle,
            file: None,
        };
        let result_item = Arc::new(Mutex::new(item));

        // ここでアイテムのアドレスを無理やり取得して、それをIDとして扱う。
        // そして内部用ハンドルを作る。
        let handle = FileHandle::new_internal_handle(result_item.clone());
        result_item.lock().unwrap().handle = Some(handle);

        // 設定から初期化を行う。
        {
            let mut item = result_item.lock().unwrap();
            match &item.setting {
                EFileAccessSetting::Write { path } => {
                    let file = fs::File::create(path).expect("Could not create a file.");
                    item.file = Some(file);
                    item.state = FileControllerState::Flying;
                }
            }
        }

        // 返す。
        result_item
    }
}

// ----------------------------------------------------------------------------
// Internal
// ----------------------------------------------------------------------------

pub struct FileIOInternal {
    /// プロキシの親元。ほかのところでは全部Weakタイプで共有する。
    original_proxy: Option<FileIOProxyPtr>,
    /// 初期設定
    initial_setting: FileIOSetting,
    /// ファイル制御のマップ
    file_map: HashMap<EFileAccessSetting, FileControllerPtr>,
}

impl FileIOInternal {
    /// 内部制御インスタンスの生成
    fn new(setting: FileIOSetting) -> Self {
        Self {
            original_proxy: None,
            initial_setting: setting.clone(),
            file_map: Default::default(),
        }
    }

    pub fn create_handle(&mut self, setting: EFileAccessSetting) -> FileHandle {
        // もし同じ設定のファイルがあるかを確認する。
        if self.file_map.contains_key(&setting) {
            let file = self.file_map.get(&setting).unwrap().clone();
            return file.lock().unwrap().create_handle();
        }

        // なければ新規で作る。
        let key = setting.clone();
        let item = FileController::new(setting);
        let handle = item.lock().unwrap().create_handle();

        // マップに入れる。
        // 後はSystemのTickがなんとかやってくれる。
        self.file_map.insert(key, item.clone());
        handle
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
