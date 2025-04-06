use std::marker::PhantomData;
use std::thread::{self, Thread};
use std::{cell::UnsafeCell, mem::MaybeUninit, sync::atomic::AtomicBool};

use std::sync::atomic::Ordering::*;

pub struct Sender<'a, T> {
    channel: &'a Channel<T>,
    receiving_thread: Thread,
}

pub struct Receiver<'a, T> {
    channel: &'a Channel<T>,
    _no_send: PhantomData<*const ()>,
}

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

    // 生存期間 'a によって、Sender と Receiver オブジェクトが Channel の生存期間だけ Channel を借用することを明示
    // Sender と Receiver が存在する限り、呼び出し側は Channel を借用したり移動したりすることができない
    // pub fn split(&mut self) -> (Sender<T>, Receiver<T>) { ... } のように生存期間を省略可能
    pub fn split<'a>(&'a mut self) -> (Sender<'a, T>, Receiver<'a, T>) {
        // 排他参照で受け取った self を 2 つの共有参照に分割し、Sender 型と Receiver 型でラップ
        // Self::new で新たな空のチャネルを作って上書きすることで、古いチャネルをドロップし、未定義動作を防ぐ
        *self = Self::new();
        (
            Sender {
                channel: self,
                receiving_thread: thread::current(), // 受信スレッドは現在のスレッドに留まる
            },
            Receiver {
                channel: self,
                _no_send: PhantomData,
            },
        )
    }
}

impl<T> Sender<'_, T> {
    pub fn send(self, message: T) {
        unsafe { (*self.channel.message.get()).write(message) };
        self.channel.ready.store(true, Release);
        self.receiving_thread.unpark();
    }
}

impl<T> Receiver<'_, T> {
    pub fn receive(self) -> T {
        // send 以外の誰かが unpark することもありうるため、ready フラグによるチェックが必要
        while !self.channel.ready.swap(false, Acquire) {
            thread::park();
        }
        unsafe { (*self.channel.message.get()).assume_init_read() }
    }
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            unsafe { self.message.get_mut().assume_init_drop() }
        }
    }
}
