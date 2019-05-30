use slog::Drain;
use slog_async::Async;
use std::fmt::Debug;

pub trait AsyncableDrain: Drain + Send + 'static
where
    Self::Err: Debug,
{
    fn async(self) -> Async;
}

impl<D: Drain + Send + 'static> AsyncableDrain for D
where
    D::Err: Debug,
{
    fn async(self) -> Async {
        Async::default(self.fuse())
    }
}
