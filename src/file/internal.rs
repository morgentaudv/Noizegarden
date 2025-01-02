use crate::file::handle::FileHandle;
use crate::file::{FileController, FileControllerPtr};
use std::io;
use std::io::{BufWriter, Write};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::MutexGuard;

/// [`FileHandle`]が書き込み可能な場合に、内容を書き込みするためのアダプター構造体。
///
/// めちゃくちゃめんどいけど、Workaroundとして`NonNull<[u8]>`を積極的に活用しよう。
///
/// 1. MutexGuardやBufWriterなどのようなライフタイムをもつものをBox化する。
/// 2. サイズを図って、むりやり`[u8]`に変換してアドレスだけを保持させる。
/// 3. `Drop`から適切なタイプに戻して、Dropする。
pub struct FileWriter<'a> {
    phantom: PhantomData<&'a FileHandle>,
    controller: FileControllerPtr,
    /// `Box<MutexGuard<'a, FileController>>`を差す。
    locked: Option<NonNull<MutexGuard<'static, FileController>>>,
    /// `Option<BufWriter<&'a std::fs::File>>`を差す。
    /// WriteやFlushする際に本来のタイプに戻す。
    buf_writer: Option<NonNull<BufWriter<&'static std::fs::File>>>,
}

unsafe impl Sync for FileWriter<'_> {}

impl FileWriter<'_> {
    pub fn new(controller: FileControllerPtr) -> Box<Self> {
        // Self-reference structをつくることになる。。。
        let result = Self {
            phantom: Default::default(),
            controller,
            // 初期化しないままにする。
            locked: None,
            buf_writer: None,
        };

        // こんな方法やってたまるか
        let mut boxed = Box::new(result);
        unsafe {
            // lockedとbuf_writerを[u8]に変換する。
            // ManuallyDropを使って、Dropしないように。。
            let mut locked = ManuallyDrop::new(Box::new(boxed.controller.lock().unwrap()));
            let mut buf_writer = ManuallyDrop::new(Box::new(BufWriter::new(locked.file.as_ref().unwrap())));

            // アドレスはBoxのなかの方がほしい。
            // こうやって無理やりLeakできる。
            let locked_pointer = locked.deref().deref() as *const _ as *mut _;
            let buf_writer_pointer = buf_writer.deref().deref() as *const _ as *mut _;
            boxed.locked = Some(NonNull::new_unchecked(locked_pointer));
            boxed.buf_writer = Some(NonNull::new_unchecked(buf_writer_pointer));
        }

        // controllerへのロックはすでにやっている状態になっている。
        // あとでDropで解除しなきゃならない。
        boxed
    }
}

impl io::Write for FileWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // buf_writerを生かして、書かせる。
        let mut addr = self.buf_writer.as_ref().unwrap().as_ptr();
        let buf_writer = unsafe { &mut *addr };
        buf_writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        // buf_writerを生かして、書かせる。
        let mut addr = self.buf_writer.as_ref().unwrap().as_ptr();
        let buf_writer = unsafe { &mut *addr };
        buf_writer.flush()
    }
}

impl io::Seek for FileWriter<'_> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        // buf_writerを生かして、書かせる。
        let mut addr = self.buf_writer.as_ref().unwrap().as_ptr();
        let buf_writer = unsafe { &mut *addr };
        buf_writer.seek(pos)
    }
}

impl Drop for FileWriter<'_> {
    fn drop(&mut self) {
        // まずbuf_writerからドロップする。
        // selfにあるアドレスはそのままにしていい。
        {
            let addr = self.buf_writer.as_ref().unwrap().as_ptr();
            let mut addr = ManuallyDrop::new(unsafe { Box::from_raw(addr) });
            unsafe { ManuallyDrop::drop(&mut addr) };

            self.buf_writer = None;
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
