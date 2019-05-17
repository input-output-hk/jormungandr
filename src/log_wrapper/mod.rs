pub mod logger;

pub use slog::Level;

/// Logs a message at the debug level.
///
/// ```rust
/// let (err_info, port) = ("No connection", 22);
///
/// debug!("Error: {} on port {}", err_info, port);
/// debug!("App Error: {}, Port: {}", err_info, 22 ; user = "user1", thread = "17");
/// ```
#[macro_export]
macro_rules!debug  {
    ($msg:expr $(, $params:expr)* $(,)*) => { log!(level: slog::Level::Debug, $msg, $($params, )*)};
}

/// Logs a message at the error level.
///
/// ```rust
/// let (err_info, port) = ("No connection", 22);
///
/// error!("Error: {} on port {}", err_info, port);
/// error!("App Error: {}, Port: {}", err_info, 22 ; user = "user1", thread = "17");
/// ```
#[macro_export]
macro_rules!error {
    ($msg:expr $(, $params:expr)* $(,)*) => { log!(level: slog::Level::Error, $msg, $($params, )*)};
}

/// Logs a message at the info level.
///
/// ```rust
/// let (err_info, port) = ("No connection", 22);
///
/// info!("Error: {} on port {}", err_info, port);
/// info!("App Error: {}, Port: {}", err_info, 22 ; user = "user1", thread = "17");
/// ```
#[macro_export]
macro_rules!info{
    ($msg:expr $(, $params:expr)* $(,)*) => { log!(level: slog::Level::Info, $msg, $($params, )*)};
}

/// Logs a message at the trace level.
///
/// ```rust
/// let (err_info, port) = ("No connection", 22);
///
/// trace!("Error: {} on port {}", err_info, port);
/// trace!("App Error: {}, Port: {}", err_info, 22 ; user = "user1", thread = "17");
/// ```
#[macro_export]
macro_rules!trace {
    ($msg:expr $(, $params:expr)* $(,)*) => { log!(level: slog::Level::Trace, $msg, $($params, )*)};
}

/// Logs a message at the warn level.
///
/// ```rust
/// let (err_info, port) = ("No connection", 22);
///
/// warn!("Error: {} on port {}", err_info, port);
/// warn!("App Error: {}, Port: {}", err_info, 22 ; user = "user1", thread = "17");
/// ```
#[macro_export]
macro_rules!warn {
    ($msg:expr $(, $params:expr)* $(,)*) => { log!(level: slog::Level::Warning, $msg, $($params, )*)};
}

/// Standard logging macros.
/// ```rust
/// let (err_info, port) = ("No connection", 22);
///
/// log!(level: log::Level::Error, "Error: {} on port {}", err_info, port;
///   user = "Loki"
///   location = "Fólkvangr"
///   );
/// ```
#[macro_export]
macro_rules!log {
    (level: $lvl:expr, $msg:expr $(, $params:expr)* $(,)*) => {
        $crate::log_wrapper::logger::with_logger(|l|slog::log!(l, $lvl, "", $msg, $($params,)* ))
    };
}
