use std::thread;

use channel::Channel;

mod channel;
mod spinlock;

fn main() {
    let channel = Channel::new();
    let t = thread::current();
    thread::scope(|s| {
        s.spawn(|| {
            channel.send("hello world!");
            t.unpark();
        });
        while !channel.is_ready() {
            thread::park();
        }
        assert_eq!(channel.receive(), "hello world!");
        // assert_eq!(channel.receive(), "hello world!"); // panic!
    });
}
