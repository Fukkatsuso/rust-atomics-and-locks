use std::thread;

use channel::channel;

mod channel;
mod spinlock;

fn main() {
    thread::scope(|s| {
        let (sender, receiver) = channel();
        let t = thread::current();
        s.spawn(move || {
            sender.send("hello world!");
            // sender.send("hello world!"); // compile error!
            t.unpark();
        });
        while !receiver.is_ready() {
            thread::park();
        }
        assert_eq!(receiver.receive(), "hello world!");
        // assert_eq!(receiver.receive(), "hello world!"); // compile error!
    });
}
