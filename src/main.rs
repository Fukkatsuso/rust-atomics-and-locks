use std::thread;

use channel::Channel;

mod channel;
mod spinlock;

fn main() {
    let mut channel = Channel::new();
    thread::scope(|s| {
        let (sender, receiver) = channel.split();
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
