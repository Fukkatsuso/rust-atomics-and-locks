use std::{cell::UnsafeCell, mem::MaybeUninit, sync::atomic::AtomicBool};

use std::sync::atomic::Ordering::*;

pub struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

// 少なくとも T が Send であれば、このチャネルをスレッド間で共有しても安全だ、ということをコンパイラに示す
unsafe impl<T> Sync for Channel<T> where T: Send {}

impl<T> Channel<T> {
    pub const fn new() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
        }
    }

    /// 安全性:1 度しか呼んではいけない !
    pub unsafe fn send(&self, message: T) {
        (*self.message.get()).write(message);
        self.ready.store(true, Release);
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Acquire)
    }

    /// 安全性:このメソッドは 1 度だけ呼ぶこと。
    /// また、is_ready() が true を返した場合にだけ呼ぶこと。
    pub unsafe fn receive(&self) -> T {
        (*self.message.get()).assume_init_read()
    }
}
