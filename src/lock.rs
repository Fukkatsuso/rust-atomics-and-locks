use std::sync::atomic::Ordering::*;
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
            // 起こされたときは state が 0 なので、自分で 2 に戻す
            while self.state.swap(2, Acquire) != 0 {
                wait(&self.state, 2);
            }
        }
        MutexGuard { mutex: self }
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
