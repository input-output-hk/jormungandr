#![warn(clippy::all)]
pub mod jcli;
pub mod jormungandr;
pub mod testing;
pub mod utils;

macro_rules! cond_println {
    ($cond:expr, $($arg:tt)*) => {
        if $cond {
            println!($($arg)*);
        }
    };
}

// hack to enable cond_println across entire crate
pub(crate) use cond_println;
