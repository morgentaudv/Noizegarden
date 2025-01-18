use crate::file::handle::FileHandle;
use crate::file::internal::EInternalData;
use crate::file::{FileController, FileControllerPtr};
use std::io;
use std::io::{BufReader, Seek, SeekFrom};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::MutexGuard;

/// [`FileHandle`]が読み込み可能な場合に、ファイルの中から内容を読み込めるためのアダプター構造体。
///
/// めちゃくちゃめんどいけど、Workaroundとして`NonNull<[u8]>`を積極的に活用しよう。
///
/// 1. MutexGuardやBufWriterなどのようなライフタイムをもつものをBox化する。
/// 2. サイズを図って、むりやり`[u8]`に変換してアドレスだけを保持させる。
/// 3. `Drop`から適切なタイプに戻して、Dropする。
pub struct FileReader<'a> {
    /// Handleが生きている時の間だけ生かせる必要がある。
    phantom: PhantomData<&'a FileHandle>,
    /// Fileが途中削除されないように保持する必要がある。
    controller: FileControllerPtr,
    /// `MutexGuard<'a, FileController>`を差す。
    locked: Option<NonNull<MutexGuard<'static, FileController>>>,
    /// `Option<BufReader<&'a std::fs::File>>`を差す。
    buf_reader: Option<NonNull<BufReader<&'static std::fs::File>>>,
    /// この構造体の挙動の設定
    setting: FileReaderSetting,
}

#[derive(Debug, Clone)]
pub struct FileReaderSetting {
    /// [`FileReader`]がDropしたらファイルのSeekを最初に戻すか？
    pub seek_to_first_when_drop: bool,
}

unsafe impl Sync for FileReader<'_> {}

impl FileReader<'_> {
    pub fn new(controller: FileControllerPtr, setting: FileReaderSetting) -> Box<Self> {
        let result = Self {
            phantom: Default::default(),
            controller,
            // 初期化しないままにする。
            locked: None,
            buf_reader: None,
            setting,
        };

        // こんな方法やってたまるか
        let mut boxed = Box::new(result);
        unsafe {
            // lockedとbuf_writerを[u8]に変換する。
            // ManuallyDropを使って、Dropしないように。。
            let locked = ManuallyDrop::new(Box::new(boxed.controller.lock().unwrap()));
            let buf_reader = match locked.internal.as_ref().unwrap() {
                EInternalData::Read { file } => {
                    ManuallyDrop::new(Box::new(BufReader::new(file)))
                }
                _ => unreachable!("Unexpected branch"),
            };

            // アドレスはBoxのなかの方がほしい。
            // こうやって無理やりLeakできる。
            let locked_pointer = locked.deref().deref() as *const _ as *mut _;
            let buf_reader_pointer = buf_reader.deref().deref() as *const _ as *mut _;
            boxed.locked = Some(NonNull::new_unchecked(locked_pointer));
            boxed.buf_reader = Some(NonNull::new_unchecked(buf_reader_pointer));
        }

        // controllerへのロックはすでにやっている状態になっている。
        // あとでDropで解除しなきゃならない。
        boxed
    }
}

impl io::Read for FileReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // buf_readerを生かして、書かせる。
        let addr = self.buf_reader.as_ref().unwrap().as_ptr();
        let buf_reader = unsafe { &mut *addr };
        buf_reader.read(buf)
    }
}

impl io::Seek for FileReader<'_> {
    fn seek(&mut self, style: io::SeekFrom) -> io::Result<u64> {
        // buf_readerを生かして、書かせる。
        let addr = self.buf_reader.as_ref().unwrap().as_ptr();
        let buf_reader = unsafe { &mut *addr };
        buf_reader.seek(style)
    }
}

impl Drop for FileReader<'_> {
    fn drop(&mut self) {
        // 設定によって戻す。
        if self.setting.seek_to_first_when_drop {
            self.seek(SeekFrom::Start(0)).unwrap();
        }

        // まずbuf_writerからドロップする。
        // selfにあるアドレスはそのままにしていい。
        {
            let addr = self.buf_reader.as_ref().unwrap().as_ptr();
            let mut addr = ManuallyDrop::new(unsafe { Box::from_raw(addr) });
            unsafe { ManuallyDrop::drop(&mut addr) };

            self.buf_reader = None;
        }

        // 次にlockedをドロップする。
        {
            let addr = self.locked.as_ref().unwrap().as_ptr();
            let mut addr = ManuallyDrop::new(unsafe { Box::from_raw(addr) });
            unsafe { ManuallyDrop::drop(&mut addr) };

            self.locked = None;
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
