use std::sync::{Arc, Mutex, OnceLock, Weak};
use serde::{Deserialize, Serialize};

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

pub struct FileIOInternal {
    /// プロキシの親元。ほかのところでは全部Weakタイプで共有する。
    original_proxy: Option<FileIOProxyPtr>,
    /// 初期設定
    initial_setting: FileIOSetting,
}

impl FileIOInternal {
    /// 内部制御インスタンスの生成
    fn new(setting: FileIOSetting) -> Self {
        Self {
            original_proxy: None,
            initial_setting: setting.clone(),
        }
    }
}

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
}

type FileIOProxyPtr = Arc<Mutex<FileIOProxy>>;
pub type FileIOProxyWeakPtr = Weak<Mutex<FileIOProxy>>;

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
