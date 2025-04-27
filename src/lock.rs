use std::sync::atomic::Ordering::*;
use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::AtomicU32,
};

use atomic_wait::{wait, wake_all, wake_one};

pub struct Mutex<T> {
    /// 0: unlocked
    /// 1: locked
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
        unsafe { &*self.mutex.value.get() }
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
        // state を 1 (locked) にセットする
        while self.state.swap(1, Acquire) == 1 {
            // すでにロックされていたら、state が 1 でなくなる (unlocked) まで待機する
            wait(&self.state, 1);
        }
        MutexGuard { mutex: self }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        // state を 0 (unlocked) に戻す
        self.mutex.state.store(0, Release);
        // 待機中のスレッドがあればその 1 つを起こす
        wake_one(&self.mutex.state);
    }
}
