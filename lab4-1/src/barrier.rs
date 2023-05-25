use std::{
    collections::HashMap,
    sync::{
        mpsc::{channel, sync_channel, Receiver, Sender, SyncSender},
        Condvar, Mutex,
    },
    thread::{self, JoinHandle},
    vec,
};

#[derive(Debug)]
enum BarrierState {
    Closed,
    Open,
}

#[derive(Debug)]
pub struct ClassicBarrier {
    state: Mutex<BarrierState>,
    state_cv: Condvar,
    nthread: u32,
    waiting: Mutex<u32>,
    waiting_cv: Condvar,
}

impl ClassicBarrier {
    pub fn new(nthread: u32) -> Self {
        Self {
            state: Mutex::new(BarrierState::Open),
            state_cv: Condvar::new(),
            nthread,
            waiting: Mutex::new(0),
            waiting_cv: Condvar::new(),
        }
    }

    pub fn wait(&self) {
        {
            let mut state = self.state.lock().unwrap();

            /* block while state is open */
            while let BarrierState::Closed = *state {
                state = self.state_cv.wait(state).unwrap();
            }
        }

        /* increase waiting count */
        let mut waiting = self.waiting.lock().unwrap();
        *waiting = *waiting + 1;

        /* block if not all thread are in wait() */
        if *waiting != self.nthread {
            waiting = self.waiting_cv.wait(waiting).unwrap();
        }

        if *waiting == self.nthread {
            /* decrease waiting count */
            *waiting -= 1;

            let mut state = self.state.lock().unwrap();
            *state = BarrierState::Closed;

            self.waiting_cv.notify_all();
        } else {
            /* decrease waiting count */
            *waiting -= 1;
        }

        if *waiting == 0 {
            let mut state = self.state.lock().unwrap();
            *state = BarrierState::Open;
            self.state_cv.notify_all();
        }
    }
}

pub struct ChannelBarrier {
    send_pipes: Vec<Sender<usize>>,
    recv_pipes: HashMap<usize, Receiver<usize>>,
    nthread: usize,
}

pub struct ChannelWaiter {
    senders: Vec<Sender<usize>>,
    receiver: Receiver<usize>,
    nthread: usize,
    id: usize,
}

impl ChannelBarrier {
    pub fn new(nthread: usize) -> Self {
        let mut sender = vec![];
        let mut receiver = HashMap::new();

        for id in 0..nthread {
            let (s, r) = channel();

            sender.push(s);
            receiver.insert(id, r);
        }

        Self {
            send_pipes: sender,
            recv_pipes: receiver,
            nthread,
        }
    }

    pub fn get_waiter(&mut self, id: usize) -> ChannelWaiter {
        let mut senders = vec![];
        for sender in &self.send_pipes {
            senders.push(sender.clone());
        }
        ChannelWaiter {
            senders,
            receiver: self.recv_pipes.remove(&id).unwrap(),
            nthread: self.nthread,
            id,
        }
    }
}

impl ChannelWaiter {
    pub fn wait(&self) {
        for sender in &self.senders {
            sender.send(self.id).unwrap();
        }

        for _ in 0..self.nthread {
            self.receiver.recv().unwrap();
        }
    }
}

pub struct ThreadBarrier {
    nthread: usize,
    sender: SyncSender<usize>,
    receiver: HashMap<usize, Receiver<usize>>,
    handle: JoinHandle<()>,
    send_kill: Sender<()>,
}

pub struct ThreadWaiter {
    id: usize,
    sender: SyncSender<usize>,
    receiver: Receiver<usize>,
}

impl ThreadBarrier {
    pub fn new(nthread: usize) -> Self {
        let mut rs_wait = HashMap::new();
        let mut ss_thread = vec![];

        let (s_wait, r_thread) = sync_channel(0);

        for id in 0..nthread {
            let (s_thread, r_wait) = channel();

            ss_thread.push(s_thread);
            rs_wait.insert(id, r_wait);
        }

        let (s_kill, r_kill) = channel();

        Self {
            nthread,
            sender: s_wait,
            receiver: rs_wait,
            handle: thread::spawn(move || loop {
                for _ in 0..nthread {
                    r_thread.recv().unwrap();
                }

                if let Ok(_) = r_kill.try_recv() {
                    break;
                }

                for (id, s_thread) in ss_thread.iter().enumerate() {
                    s_thread.send(id).unwrap();
                }
            }),
            send_kill: s_kill,
        }
    }

    pub fn get_waiter(&mut self, id: usize) -> ThreadWaiter {
        ThreadWaiter {
            id,
            sender: self.sender.clone(),
            receiver: self.receiver.remove(&id).unwrap(),
        }
    }

    pub fn stop(self) {
        for id in 0..self.nthread {
            self.sender.send(id).unwrap();
        }
        self.send_kill.send(()).unwrap();
        self.handle.join().unwrap();
    }
}

impl ThreadWaiter {
    pub fn wait(&self) {
        self.sender.send(self.id).unwrap();
        self.receiver.recv().unwrap();
    }
}
