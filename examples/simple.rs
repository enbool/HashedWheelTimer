use std::{thread, time::Duration};

use timer::HashedWheelTimer::HashedWheelTimer as timer;

fn main() {
    println!("Timer starting...");
    let timer = timer::new(100, 16, 6000);
    timer.newTimeout(Box::new(|| {
        println!("hello");
    }), 1000);
    timer.newTimeout(Box::new(|| {
        println!("world");
    }), 2000);
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}