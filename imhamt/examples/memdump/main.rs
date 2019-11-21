#[cfg(unix)]
mod unix;

#[cfg(unix)]
use unix::run;

#[cfg(not(unix))]
fn run() {
    println!("example not supported on this platform")
}

fn main() {
    run()
}
