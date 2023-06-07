use std::{collections::{VecDeque, HashMap}, thread::{self, JoinHandle}, time::Duration};

use crossbeam::channel::{Sender, Receiver};

#[derive(Debug)]
enum WorkerState {
    Ready,
    Working,
}

fn worker<F>(id: u32, f_recv: Receiver<F>,  finish_job: Sender<u32>)
where F: FnOnce() -> () + Send + 'static {
    loop {
        let f = f_recv.recv().unwrap();

        f();

        finish_job.send(id).unwrap();
    }
}

fn scheduler<F>(wake_channel: Receiver<F>, mut pool: Scheduler<F>)
where F: FnOnce() -> () + Send + 'static {
    loop {
        crossbeam::select! {
            recv(wake_channel) -> res => {
                pool.ready_jobs.push_back(res.unwrap());
            },
            recv(pool.job_finish_recv) -> id => {
                let w = pool.workers.get_mut(&id.unwrap()).unwrap();
                w.0 = WorkerState::Ready;
            },
        }

        for (_, v) in pool.workers.iter_mut() {
            if let WorkerState::Working = v.0 { continue; }

            if let Some(f) = pool.ready_jobs.pop_front() {
                v.0 = WorkerState::Working;
                v.1.send(f).unwrap();
            }
        }
    }
}

struct Scheduler<F> {
    ready_jobs: VecDeque<F>,
    workers: HashMap<u32, (WorkerState, Sender<F>)>,
    workers_handle: HashMap<u32, JoinHandle<()>>,
    job_finish_recv: Receiver<u32>,
}

struct ThreadPool<F> {
    wake_scheduler: Sender<F>,
    scheduler_handle: JoinHandle<()>,
}

impl<F: FnOnce() -> () + Send + 'static> ThreadPool<F> {
    fn new(n_workers: u32) -> Self {
        let mut workers = HashMap::new();
        let mut workers_handle = HashMap::new();
        let (worker_done_sx, worker_done_rx) = crossbeam::channel::bounded::<u32>(0);


        for id in 0..n_workers {
            // clone job sender 
            let worker_done_sx = worker_done_sx.clone();
            let (job_sx, job_rx) = crossbeam::channel::unbounded::<F>();
            
            workers.insert(id, (WorkerState::Ready, job_sx));

            let handle = thread::spawn(move || worker(id, job_rx, worker_done_sx));

            workers_handle.insert(id, handle);
        }

        let sched = Scheduler {
            ready_jobs: VecDeque::new(),
            workers,
            workers_handle,
            job_finish_recv: worker_done_rx,
        };

        let (wake_scheduler_rx, wake_scheduler_sx) = crossbeam::channel::unbounded::<F>();

        let s = thread::spawn(move || scheduler(wake_scheduler_sx, sched));

        Self {
            wake_scheduler: wake_scheduler_rx,
            scheduler_handle: s,
        }
    }

    fn execute(&self, job: F) {
        self.wake_scheduler.send(job).unwrap();
    }
}

fn main() {
    // alloca i worker
    let threadpool = ThreadPool::new(10);
    for x in 0..100 {
        threadpool.execute(move || {
            println!("long running task {}", x);
            thread::sleep(Duration::from_millis(1000))
        })
    }
    // just to keep the main thread alive
    loop {
        thread::sleep(Duration::from_millis(1000))
    }
}
