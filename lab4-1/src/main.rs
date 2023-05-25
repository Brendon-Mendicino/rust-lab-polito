use std::{sync::Arc, thread};

use barrier::ClassicBarrier;

use crate::barrier::{ChannelBarrier, ThreadBarrier};

mod barrier;

fn main() {
    let classic_barrier = Arc::new(ClassicBarrier::new(3));

    println!("\nClassical\n");
    thread::scope(|s| {
        for i in 0..3 {
            let b = classic_barrier.clone();

            s.spawn(move || {
                for j in 0..10 {
                    b.wait();
                    println!("after barrier {} {}", i, j);
                }
            });
        }
    });

    let mut channel_barrier = ChannelBarrier::new(3);

    println!("\nChannel Barrier\n");
    thread::scope(|s| {
        for i in 0..3 {
            let w = channel_barrier.get_waiter(i as usize);

            s.spawn(move || {
                for j in 0..10 {
                    w.wait();
                    println!("after barrier {} {}", i, j);
                }
            });
        }
    });

    let mut thread_barrier = ThreadBarrier::new(3);

    println!("\nThread Barrier\n");
    thread::scope(|s| {
        for i in 0..3 {
            let w = thread_barrier.get_waiter(i as usize);

            s.spawn(move || {
                for j in 0..10 {
                    w.wait();
                    println!("after barrier {} {}", i, j);
                }
            });
        }
    });

    thread_barrier.stop();
}
