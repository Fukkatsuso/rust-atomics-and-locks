use std::sync::atomic::Ordering::*;
use std::thread;
use std::time::Duration;
use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::AtomicU32,
};

use atomic_wait::{wait, wake_all, wake_one};

pub struct Mutex<T> {
    /// 0: unlocked
    /// 1: locked, 他の待機スレッドはない
    /// 2: locked, 他に待機スレッドがある
    state: AtomicU32,
    value: UnsafeCell<T>,
}

unsafe impl<T> Sync for Mutex<T> where T: Send {}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

unsafe impl<T> Sync for MutexGuard<'_, T> where T: Sync {}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.value.get() } // &*: 生ポインタを参照に変換している
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<T> Mutex<T> {
    pub const fn new(value: T) -> Self {
        Self {
            state: AtomicU32::new(0), // unlocked
            value: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        if self.state.compare_exchange(0, 1, Acquire, Relaxed).is_err() {
            // ここに到達したということは、Mutex はすでに state 1 or 2 でロックされている
            lock_contended(&self.state);
        }
        MutexGuard { mutex: self }
    }
}

fn lock_contended(state: &AtomicU32) {
    let mut spin_count = 0;

    while state.load(Relaxed) == 1 && spin_count < 100 {
        spin_count += 1;
        std::hint::spin_loop();
    }

    if state.compare_exchange(0, 1, Acquire, Relaxed).is_ok() {
        return;
    }

    while state.swap(2, Acquire) != 0 {
        wait(state, 2);
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        if self.mutex.state.swap(0, Release) == 2 {
            // 待機中のスレッドがいる場合のみ、wake_one を呼び出す
            wake_one(&self.mutex.state);
        }
    }
}

pub struct Condvar {
    counter: AtomicU32,
}

impl Condvar {
    pub const fn new() -> Self {
        Self {
            counter: AtomicU32::new(0),
        }
    }

    pub fn notify_one(&self) {
        self.counter.fetch_add(1, Relaxed);
        wake_one(&self.counter);
    }

    pub fn notify_all(&self) {
        self.counter.fetch_add(1, Relaxed);
        wake_all(&self.counter);
    }

    pub fn wait<'a, T>(&self, guard: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
        let counter_value = self.counter.load(Relaxed);

        // ガードをドロップすることで、Mutex をアンロックする
        // ただし、後でロックするために mutex を覚えておく
        let mutex = guard.mutex;
        drop(guard);

        // カウンタ値がアンロックする前から変更されていない場合にだけ待機する
        wait(&self.counter, counter_value);

        mutex.lock()
    }
}

#[test]
fn test_condvar() {
    let mutex = Mutex::new(0);
    let condvar = Condvar::new();

    let mut wakeups = 0;

    thread::scope(|s| {
        s.spawn(|| {
            thread::sleep(Duration::from_secs(1));
            *mutex.lock() = 123;
            condvar.notify_one();
        });

        let mut m = mutex.lock();
        while *m < 100 {
            m = condvar.wait(m);
            wakeups += 1;
        }

        assert_eq!(*m, 123);
    });

    // メインスレッドが(ビジーループではなく)実際にウェイトしたことをチェック。
    // ただし、何度か誤って起こされることは許容する。
    assert!(wakeups < 10);
}
