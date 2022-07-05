use std::{sync::{atomic::{AtomicUsize, AtomicBool, AtomicU64, Ordering::*}, Condvar, Arc}, thread::{JoinHandle, self, Thread}, time::{SystemTime, UNIX_EPOCH, Duration, Instant}};
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;

use std::sync::Mutex;


static INSTANCE_COUNTER: AtomicUsize = AtomicUsize::new(0);
static WARNED_TOO_MANY_INSTANCES: AtomicBool = AtomicBool::new(false);
pub const INSTANCE_COUNT_LIMIT: usize = 64;
pub const MILLISECOND_NANOS: u64 = 1000000;

pub struct HashedWheelTimer {

    worker: Arc<Worker>,
    // workerThread: JoinHandle<()>,
    workerState: WorkerState,

    tickDuration: u128,
    wheel: Vec<HashedWheelBucket>,
    mask: u128,
    startTimeInitialized: Arc<Condvar>,
    sender: Sender<HashedWheelTimeout>,
    receiver: Receiver<HashedWheelTimeout>,
    pendingTimeouts: AtomicU64,
    maxPendingTimeouts: u64,

    startTime: Arc<Mutex<u128>>,
}

impl HashedWheelTimer {
    pub fn new_default() -> HashedWheelTimer {
        HashedWheelTimer::new(100, 512, 0)
    }

    pub fn new(tickDuration: u128, ticksPerWheel: u128, maxPendingTimeouts: u64) -> HashedWheelTimer {
        let wheel = HashedWheelTimer::createWheel(ticksPerWheel);
        let worker = Arc::new(Worker::new());
        let mask = wheel.len() as u128 - 1;

        let (sender, receiver) = channel::<HashedWheelTimeout>();
        

        if INSTANCE_COUNTER.fetch_add(1, Relaxed) > INSTANCE_COUNT_LIMIT 
                && WARNED_TOO_MANY_INSTANCES.compare_exchange(false, true,Acquire,Relaxed).is_ok() {
            println!("Too may instance");
        }

        Self{
            worker: worker,
            // workerThread: ,
            workerState: WorkerState::INIT,
            tickDuration: tickDuration,
            wheel: wheel,
            mask: mask,
            startTimeInitialized: Arc::new(Condvar::new()),
            sender: sender,
            receiver: receiver,
            pendingTimeouts: AtomicU64::new(1),
            maxPendingTimeouts: maxPendingTimeouts,
            startTime: Arc::new(Mutex::new(0)),
        }
    }

    pub fn start(&self) {
        match self.workerState {
            WorkerState::INIT => {
                println!("starting");
                thread::spawn(move || self.worker.run());
            },
            WorkerState::STARTED => println!("started"),
            WorkerState::SHUTDOWN => panic!("cannot be started once stopped"),
        };
        let startTime = self.startTime.clone();
        let startTimeInitialized = self.startTimeInitialized.clone();
        let mut started = startTime.lock().unwrap();
        while *started == 0 {
            started = startTimeInitialized.wait(started).unwrap();
        }
    }

    pub fn newTimeout(&self, task: Arc<dyn FnOnce()>, delay: u128) { // -> HashedWheelTimeout {
        let pendingTimeoutsCount = self.pendingTimeouts.fetch_add(1, Relaxed);
        if self.maxPendingTimeouts > 0 && pendingTimeoutsCount > self.maxPendingTimeouts {
            self.pendingTimeouts.fetch_sub(1, Relaxed);
            panic!("Number of pending timeouts ({}) is greater than or equal to maximum allowed pending 
                timeouts ({})", pendingTimeoutsCount, self.maxPendingTimeouts)
        }

        self.start();

        let mut deadline = currentTime() + delay - *self.startTime.lock().unwrap();

        // Guard against overflow.
        if delay > 0 && deadline < 0 {
            deadline = u128::MAX;
        }  
        let timeout = HashedWheelTimeout::new(task, deadline);
        self.sender.send(timeout);

        // return timeout;
    }

    fn createWheel(ticksPerWheel: u128) -> Vec<HashedWheelBucket> {
        let ticksPerWheel = HashedWheelTimer::normalizeTicksPerWheel(ticksPerWheel);
        let mut wheel = Vec::with_capacity(ticksPerWheel as usize);
        for i in 0..ticksPerWheel {
            wheel.push(HashedWheelBucket::new());
        }
        return wheel;
    }
    fn normalizeTicksPerWheel(ticksPerWheel: u128) -> u128 {
        let mut normalizedTicksPerWheel = 1;
        while normalizedTicksPerWheel < ticksPerWheel {
            normalizedTicksPerWheel <<= 1;
        }
        return normalizedTicksPerWheel;
    }
}

unsafe impl Send for HashedWheelTimer {}

struct Worker {
    timer: Option<HashedWheelTimer>,
    tick: u128,
}

unsafe impl Send for Worker {}
unsafe impl Sync for Worker {}

impl Worker {
    pub fn new() -> Self {
        Self {
            timer: None,
            tick: 0,
        }
    }

    pub fn init() {
        
    }

    fn run(&self) {
        if self.timer.is_none() {
            thread::sleep(Duration::from_secs(1));
        }
        match &self.timer {
            None => thread::sleep(Duration::from_secs(1)),
            Some(timer) => {
                // TODO
                let mut startTime = currentTime();
                if startTime == 0 {
                    startTime = 1;
                }
                *timer.startTime.clone().lock().unwrap() = startTime;
                // Notify the other threads waiting for the initialization at start().
                let startTimeInitialized = timer.startTimeInitialized.clone();
                startTimeInitialized.notify_all();
                while timer.workerState == WorkerState::STARTED {
                    let deadline = self.waitForNextTick(&timer);
                    if deadline > 0 {
                        let idx = self.tick & timer.mask;
                        // self.processCancelledTasks();

                        let bucket: HashedWheelBucket = timer.wheel[idx as usize];
                        self.transferTimeoutsToBuckets();
                        bucket.expireTimeouts(deadline);
                        self.tick += 1;
                    }
                }
                //TODO: worker stoped, need clear the remain job.
            },
        }
        
        
    }
    /**
     * transfer timeouts to buckets every tick.
     */
    fn transferTimeoutsToBuckets(&self) {
        // transfer only max. 100000 timeouts per tick to prevent a thread to stale the workerThread when it just
        // adds new timeouts in a loop.
        let timer = self.timer.unwrap();
        for _ in 0..100000 {
            let timeout = timer.receiver.recv().unwrap();
            if timeout.state == TimeoutState::CANCELED {
                continue;
            }
            let calculated = timeout.deadline / timer.tickDuration;
            timeout.remainingRounds = (calculated - self.tick) / timer.wheel.len() as u128;
            // Ensure we don't schedule for past.
            let ticks = calculated.max(self.tick);
            let stopIndex = ticks & timer.mask;
            let bucket: HashedWheelBucket = timer.wheel[stopIndex as usize];
            bucket.addTimeout(timeout);
        }
    }

    
    /// calculate goal nanoTime from startTime and current tick number,
    /// then wait until that goal has been reached.
    /// 
    /// @return Long.MIN_VALUE if received a shutdown request,
    /// current time otherwise (with Long.MIN_VALUE changed by +1)
    fn waitForNextTick(&self, timer: &HashedWheelTimer) -> u128 {
        let deadline = timer.tickDuration * (self.tick + 1);
        loop {
            let currentTime = currentTime() - *timer.startTime.lock().unwrap();
            let sleepTimeMs = (deadline - currentTime + 999) / 1000;

            if sleepTimeMs <= 0 {
                if currentTime == u128::MAX {
                    return u128::MAX;
                } else {
                    return currentTime;
                }
            }

            thread::sleep(Duration::from_millis(sleepTimeMs as u64));
        }
    }
}

struct HashedWheelBucket {
    inner: Vec<HashedWheelTimeout>,
}

impl HashedWheelBucket {
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
        }
    }

    pub fn addTimeout(&self, timeout: HashedWheelTimeout) {
        self.inner.push(timeout);
    }

    pub fn expireTimeouts(&self, deadline: u128) {
        let expireds: Vec<HashedWheelTimeout> = self.inner.drain_filter(|e| e.remainingRounds <= 0).collect();
        expireds.iter().for_each(|timeout| timeout.expire());
        self.inner.iter().for_each(|timeout| timeout.remainingRounds -= 1);
    }
}

struct HashedWheelTimeout {
    task: Arc<dyn FnOnce()>,
    deadline: u128,
    state: TimeoutState,
    // remainingRounds will be calculated and set by Worker.transferTimeoutsToBuckets() before the
    // HashedWheelTimeout will be added to the correct HashedWheelBucket.
    remainingRounds: u128,
    // The bucket to which the timeout was added
    // bucket: Arc<Vec<HashedWheelTimeout>>,
}

impl HashedWheelTimeout {
    pub fn new(task: Arc<dyn FnOnce()>, deadline: u128) -> Self {
        Self {
            state: TimeoutState::INIT,
            task: task,
            deadline,
            remainingRounds: 0,
        }
    }

    pub fn expire(&self) {
        (self.task)();
    }
}

#[derive(PartialEq, Eq)]
enum WorkerState {
    INIT,
    STARTED,
    SHUTDOWN,
}
#[derive(PartialEq, Eq)]
enum TimeoutState {
    INIT,
    CANCELED,
    EXPIRED,
}


fn currentTime() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
}