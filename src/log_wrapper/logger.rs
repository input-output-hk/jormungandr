use slog;
use slog::Drain;
use slog_term;
use slog_async;
pub use slog::Level;

use std::cell::RefCell;
use std::sync::Mutex;
use std::thread_local;

lazy_static! {
    static ref TOP_LOGGER: Mutex<RefCell<slog::Logger>> = {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::FullFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let log = slog::Logger::root(drain, o!());
        Mutex::new(RefCell::new(log))
    };
}

thread_local! {
  static THREAD_LOGGER : RefCell<Option<slog::Logger>>
    = RefCell::new(None);
}

/// Utility function that returns a global logger
/// in case if local one was not set. Returns `None`
/// in case if local logger is set.
fn get_global_logger() -> slog::Logger {
    let ref_cell = TOP_LOGGER.lock().unwrap();
    let cell = ref_cell.borrow();
    cell.clone()
}

/// Load thread local logger, potentially creating a new one
/// based on the global logger.
pub fn with_logger<F>(f: F)
where
    F: FnOnce(&slog::Logger),
{
    THREAD_LOGGER.with(|ref_logger| {
        let new_logger = {
            let st = ref_logger.borrow();
            match *st {
                None => {
                    let logger = get_global_logger();
                    f(&logger);
                    Some(logger)
                },
                Some(ref logger) => {
                    f(&logger);
                    None
                }
            }
        };
        new_logger.map(|logger| ref_logger.replace(Some(logger)));
        ()
    });
}

/// Update thread local logger, all logs in the
/// current thread will go to the logger.
///
/// Function receives a function that is used to update
/// the loggerr, for example, in order to add tags one
/// can use:
///
/// ```rust
/// thread::spawn(|| {
///    update_thread_logger(|l| l.new(o!("thread"=>"my thread")));
/// })
/// ```
pub fn update_thread_logger<F>(f: F)
where
    F: FnOnce(&slog::Logger) -> slog::Logger,
{
    THREAD_LOGGER.with(|ref_logger| {
        let v = ref_logger.borrow().clone();
        v.or_else(|| {
            let logger = get_global_logger();
            ref_logger.replace(Some(f(&logger)))
        });
    });
}

/// Define a global logger to be used within an application.
///
/// Note. This function do not update all thread local loggers
/// that were defined for the threads earlier.
pub fn set_global_logger(logger: slog::Logger) {
    let logger2 = logger.clone();
    let ref_cell = TOP_LOGGER.lock().unwrap();
    ref_cell.replace(logger);
    update_thread_logger(|_| logger2.clone());
}
