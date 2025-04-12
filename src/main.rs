use std::thread;

use channel::Channel;

mod arc;
mod channel;
mod spinlock;

fn main() {
    let mut channel = Channel::new();
    thread::scope(|s| {
        let (sender, receiver) = channel.split();
        s.spawn(move || {
            sender.send("hello world!");
            // sender.send("hello world!"); // compile error!
        });
        assert_eq!(receiver.receive(), "hello world!");
        // assert_eq!(receiver.receive(), "hello world!"); // compile error!
    });
}
