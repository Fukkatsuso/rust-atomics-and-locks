use std::sync::atomic::{fence, Ordering::*};
use std::usize;
use std::{ops::Deref, ptr::NonNull, sync::atomic::AtomicUsize};

struct ArcData<T> {
    ref_count: AtomicUsize,
    data: T,
}

pub struct Arc<T> {
    ptr: NonNull<ArcData<T>>,
}

unsafe impl<T: Send + Sync> Send for Arc<T> {}
unsafe impl<T: Send + Sync> Sync for Arc<T> {}

impl<T> Arc<T> {
    pub fn new(data: T) -> Arc<T> {
        Arc {
            ptr: NonNull::from(Box::leak(Box::new(ArcData {
                ref_count: AtomicUsize::new(1),
                data,
            }))),
        }
    }

    fn data(&self) -> &ArcData<T> {
        unsafe { self.ptr.as_ref() }
    }

    pub fn get_mut(arc: &mut Self) -> Option<&mut T> {
        if arc.data().ref_count.load(Relaxed) == 1 {
            fence(Acquire);
            // 安全性:Arc は 1 つしかないので、他の何もデータにアクセスできない。
            // その Arc に対してこのスレッドが排他アクセス権限を持っている。
            unsafe { Some(&mut arc.ptr.as_mut().data) }
        } else {
            None
        }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data().data
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Relaxed) > usize::MAX / 2 {
            std::process::abort();
        }
        Arc { ptr: self.ptr }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Release) == 1 {
            fence(Acquire);
            unsafe {
                drop(Box::from_raw(self.ptr.as_ptr()));
            }
        }
    }
}

#[test]
fn test() {
    static NUM_DROPS: AtomicUsize = AtomicUsize::new(0);

    struct DetectDrop;

    impl Drop for DetectDrop {
        fn drop(&mut self) {
            NUM_DROPS.fetch_add(1, Relaxed);
        }
    }

    // 文字列と DetectDrop を保持するオブジェクトを共有する
    // Arc を 2 つ作る。DetectDrop でいつドロップされたかわかる
    let x = Arc::new(("hello", DetectDrop));
    let y = x.clone();

    // x をもう 1 つのスレッドに送り、そこで使う
    let t = std::thread::spawn(move || {
        assert_eq!(x.0, "hello");
    });

    // 同時に y はこちらで使えるはず
    assert_eq!(y.0, "hello");

    // スレッドが終了するのを待機
    t.join().unwrap();

    // Arc xはここまででドロップされているはず
    // まだ y があるので、オブジェクトはまだドロップされない
    assert_eq!(NUM_DROPS.load(Relaxed), 0);

    // 残った `Arc` をドロップ
    drop(y);

    // `y`もドロップしたので、オブジェクトもドロップされたはず
    assert_eq!(NUM_DROPS.load(Relaxed), 1);
}
