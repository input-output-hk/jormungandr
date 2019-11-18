extern crate imhamt;
extern crate jemalloc_ctl;
extern crate jemallocator;

use jemalloc_ctl::{epoch, stats};
use std::collections::hash_map::DefaultHasher;
use std::thread;
use std::time::Duration;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use imhamt::Hamt;

fn statprint() {
    let allocated = stats::allocated::read().unwrap();
    let resident = stats::resident::read().unwrap();
    println!("{} bytes allocated/{} bytes resident", allocated, resident);
}

fn main() {
    let mut h: Hamt<DefaultHasher, u32, u32> = Hamt::new();

    println!("adding 100000 entries");

    for i in 0..100000 {
        h = h.insert(i, i).unwrap();
    }

    let mut h2 = h.clone();

    epoch::advance().unwrap();

    statprint();
    thread::sleep(Duration::from_secs(10));

    println!("adding 100000 entries");

    for i in 100000..200000 {
        h2 = h2.insert(i, i).unwrap();
    }

    epoch::advance().unwrap();

    statprint();
    thread::sleep(Duration::from_secs(10));

    epoch::advance().unwrap();

    statprint();
}
