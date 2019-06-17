use slog::Drain;
use slog_async::Async;
use std::fmt::Debug;

pub trait AsyncableDrain: Drain + Send + 'static
where
    Self::Err: Debug,
{
    fn async(self) -> Async;
}

const EVENT_BUFFER_SIZE: usize = 1024;

impl<D: Drain + Send + 'static> AsyncableDrain for D
where
    D::Err: Debug,
{
    fn async(self) -> Async {
        Async::new(self.fuse()).chan_size(EVENT_BUFFER_SIZE).build()
    }
}
