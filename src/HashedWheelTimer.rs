use std::{sync::{atomic::{AtomicUsize, AtomicBool, AtomicU64, Ordering::*}, Condvar, Arc}, thread::{JoinHandle, self, Thread}, time::{SystemTime, UNIX_EPOCH, Duration, Instant}, borrow::{BorrowMut, Borrow}, cell::RefCell, rc::Rc};
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;

use std::sync::Mutex;


static INSTANCE_COUNTER: AtomicUsize = AtomicUsize::new(0);
static WARNED_TOO_MANY_INSTANCES: AtomicBool = AtomicBool::new(false);
pub const INSTANCE_COUNT_LIMIT: usize = 64;
pub const MILLISECOND_NANOS: u64 = 1000000;

pub struct HashedWheelTimer {

    worker: Arc<Mutex<Worker>>,
    // workerThread: JoinHandle<()>,
    
    sender: Sender<HashedWheelTimeout>,
    
}

impl HashedWheelTimer {
    pub fn new_default() -> HashedWheelTimer {
        HashedWheelTimer::new(100, 512, 0)
    }

    pub fn new(tickDuration: u128, ticksPerWheel: u128, maxPendingTimeouts: u64) -> HashedWheelTimer {
        let (sender, receiver) = channel::<HashedWheelTimeout>();

        let worker = Arc::new(Mutex::new(Worker::new(tickDuration, ticksPerWheel, maxPendingTimeouts, receiver)));     
        
        if INSTANCE_COUNTER.fetch_add(1, Relaxed) > INSTANCE_COUNT_LIMIT 
                && WARNED_TOO_MANY_INSTANCES.compare_exchange(false, true,Acquire,Relaxed).is_ok() {
            println!("Too may instance");
        }

        Self{
            worker,           
            sender,            
        }
    }

    pub fn start(&self) {
        let mut worker = self.worker.lock().unwrap();
        match worker.workerState {
            WorkerState::INIT => {
                worker.workerState = WorkerState::STARTED;
                let mut startTime = currentTime();
                if startTime == 0 {
                    startTime = 1;
                }
                worker.startTime = startTime;

                let worker = self.worker.clone();
                println!("started: {}", startTime);
                thread::spawn(move || worker.lock().unwrap().run());
            },
            WorkerState::STARTED => {},// println!("started"),
            WorkerState::SHUTDOWN => panic!("cannot be started once stopped"),
        };
        while worker.startTime == 0 {
            thread::sleep(Duration::from_millis(1));
            // started = startTimeInitialized.wait(started).unwrap();
        }
    }

    pub fn newTimeout(&self, task: Box<dyn Fn()>, delay: u128) { // -> HashedWheelTimeout {
        {
            let worker = self.worker.lock().unwrap();
            let pendingTimeoutsCount = worker.pendingTimeouts.fetch_add(1, Relaxed);
            if worker.maxPendingTimeouts > 0 && pendingTimeoutsCount > worker.maxPendingTimeouts {
                worker.pendingTimeouts.fetch_sub(1, Relaxed);
                panic!("Number of pending timeouts ({}) is greater than or equal to maximum allowed pending 
                    timeouts ({})", pendingTimeoutsCount, worker.maxPendingTimeouts)
            }
        }
        self.start();

        let mut deadline = currentTime() + delay - self.worker.lock().unwrap().startTime;

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
    workerState: WorkerState,

    tickDuration: u128,
    wheel: Vec<HashedWheelBucket>,
    mask: u128,
    startTimeInitialized: Arc<Condvar>,
    receiver: Receiver<HashedWheelTimeout>,
    pendingTimeouts: AtomicU64,
    maxPendingTimeouts: u64,

    startTime: u128,
    tick: u128,
}

unsafe impl Send for Worker {}
unsafe impl Sync for Worker {}

impl Worker {
    pub fn new(tickDuration: u128, ticksPerWheel: u128, maxPendingTimeouts: u64, receiver: Receiver<HashedWheelTimeout>) -> Self {
        let wheel = HashedWheelTimer::createWheel(ticksPerWheel);
        
        let mask = wheel.len() as u128 - 1;

        Self {
            workerState: WorkerState::INIT,
            tickDuration: tickDuration,
            wheel: wheel,
            mask: mask,
            startTimeInitialized: Arc::new(Condvar::new()),
            receiver: receiver,
            pendingTimeouts: AtomicU64::new(1),
            maxPendingTimeouts: maxPendingTimeouts,
            startTime: 0,
            tick: 0,
        }
    }

    pub fn init() {
        
    }

    fn run(&mut self) {
        
        // let mut startTime = currentTime();
        // if startTime == 0 {
        //     startTime = 1;
        // }
        // self.startTime = startTime;
        // Notify the other threads waiting for the initialization at start().
        // let startTimeInitialized = self.startTimeInitialized.clone();
        // startTimeInitialized.notify_all();
        
        while self.workerState == WorkerState::STARTED {
            let deadline = self.waitForNextTick();
            if deadline > 0 {
                
                // self.processCancelledTasks();
                self.transferTimeoutsToBuckets();
                
                let idx = self.tick & self.mask;
                
                self.wheel[idx as usize].expireTimeouts(deadline);                
                // bucket.expireTimeouts(deadline);
                
                self.tick += 1;
            }
        }
        //TODO: worker stoped, need clear the remain job.      
        
    }
    /**
     * transfer timeouts to buckets every tick.
     */
    fn transferTimeoutsToBuckets(&mut self) {
        // transfer only max. 100000 timeouts per tick to prevent a thread to stale the workerThread when it just
        // adds new timeouts in a loop.
        for _ in 0..100000 {
            let mut timeout = self.receiver.try_recv();
            match timeout {
                Ok(mut timeout) => {
                    if timeout.state == TimeoutState::CANCELED {
                        continue;
                    }
                    let calculated = timeout.deadline / self.tickDuration;
                    timeout.remainingRounds = (calculated - self.tick) / self.wheel.len() as u128;
                    // Ensure we don't schedule for past.
                    let ticks = calculated.max(self.tick);
                    let stopIndex = ticks & self.mask;
                    self.wheel[stopIndex as usize].addTimeout(timeout);
                },
                Err(e) => return,
            }
            
        }
    }

    
    /// calculate goal nanoTime from startTime and current tick number,
    /// then wait until that goal has been reached.
    /// 
    /// @return Long.MIN_VALUE if received a shutdown request,
    /// current time otherwise (with Long.MIN_VALUE changed by +1)
    fn waitForNextTick(&self) -> u128 {
        let deadline = self.tickDuration * (self.tick + 1);
        loop {
            let mut currentTime = currentTime();
            currentTime -= self.startTime;
            let sleepTimeMs = (deadline + 999999 - currentTime) / 1000000;

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

    pub fn addTimeout(&mut self, timeout: HashedWheelTimeout) {
        self.inner.push(timeout);
    }

    pub fn expireTimeouts(&mut self, deadline: u128) {
        let expireds: Vec<HashedWheelTimeout> = self.inner.drain_filter(|e| e.remainingRounds <= 0).collect();
        expireds.iter().for_each(|timeout| timeout.expire());
        self.inner.iter_mut().for_each(|timeout| timeout.remainingRounds -= 1);
    }
}

struct HashedWheelTimeout {
    task: Box<dyn Fn()>,
    deadline: u128,
    state: TimeoutState,
    // remainingRounds will be calculated and set by Worker.transferTimeoutsToBuckets() before the
    // HashedWheelTimeout will be added to the correct HashedWheelBucket.
    remainingRounds: u128,
    // The bucket to which the timeout was added
    // bucket: Arc<Vec<HashedWheelTimeout>>,
}

impl HashedWheelTimeout {
    pub fn new(task: Box<dyn Fn()>, deadline: u128) -> Self {
        Self {
            state: TimeoutState::INIT,
            task: task,
            deadline,
            remainingRounds: 0,
        }
    }

    pub fn expire(&self) {
        print!("Task running: {}  ", currentTime());
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