use timer::HashedWheelTimer::HashedWheelTimer as timer;

fn main() {
    let timer = timer::new(100, 16, 6000);
    timer.newTimeout(|| {
        println!("{hello}");
    }, 1000);
}