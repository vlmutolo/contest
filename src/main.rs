use crate::{atomic::SharedAtomic, unsync::SharedUnsync};
use rand::{rngs::SmallRng, Rng, SeedableRng};

fn main() {
    let unsync = SharedUnsync::new();
    let atomic = SharedAtomic::new();

    let start = std::time::Instant::now();

    let mut threads = Vec::new();
    for _ in 0..32 {
        let unsync = unsync.clone();
        let atomic = atomic.clone();

        let handle = std::thread::spawn(move || {
            let mut rng = SmallRng::from_entropy();
            for _ in 0..256 {
                let n: u64 = rng.gen();
                for _ in 0..2048 {
                    do_xors(n, &atomic, &unsync);
                }
            }
        });

        threads.push(handle);
    }

    threads.into_iter().for_each(|t| t.join().unwrap());

    println!("unsync: {:064b}", unsync.get());
    println!("atomic: {:064b}", atomic.get());
    println!("took {:.0?}", start.elapsed());
}

// Try commenting out `atomic.fetch_xor(n);`.
fn do_xors(n: u64, atomic: &SharedAtomic, unsync: &SharedUnsync) {
    atomic.fetch_xor(n);
    unsync.fetch_xor(n);
}

/// In order to participate in our race, you must provide
/// methods to create yourself, perform xors, and inspect
/// your value at the end to check against 0.
trait Race: Clone + Send + Sync {
    fn new() -> Self;
    fn get(&self) -> u64;
    fn fetch_xor(&self, other: u64);
}

/// This module contains a type that erroneously implements
/// Send and Sync without actually synchronising data access.
/// Let's see what happens.
mod unsync {
    use super::Race;
    use std::{cell::UnsafeCell, sync::Arc};

    #[derive(Clone)]
    pub struct SharedUnsync(Arc<UnsafeCell<u64>>);

    impl Race for SharedUnsync {
        fn new() -> Self {
            Self(Arc::new(UnsafeCell::new(0)))
        }

        fn get(&self) -> u64 {
            unsafe { *self.0.get() }
        }

        fn fetch_xor(&self, other: u64) {
            // SAFETY: very unsafe.
            unsafe { *self.0.get() ^= other }
        }
    }

    // SAFETY: still unsafe.
    unsafe impl Send for SharedUnsync {}
    unsafe impl Sync for SharedUnsync {}
}

/// The `atomic` module uses processor-intrinsics to do
/// fetch-add atomically.
mod atomic {
    use super::Race;
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    };

    #[derive(Clone)]
    pub struct SharedAtomic(Arc<AtomicU64>);

    impl Race for SharedAtomic {
        fn new() -> Self {
            Self(Arc::new(AtomicU64::new(0)))
        }

        fn get(&self) -> u64 {
            self.0.load(Ordering::Relaxed)
        }

        fn fetch_xor(&self, other: u64) {
            self.0.fetch_xor(other, Ordering::Relaxed);
        }
    }
}
