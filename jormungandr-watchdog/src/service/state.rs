//! # State and StateSaver
//!
//! the `State` is behaving just like a `RwLock` from the standard
//! library but with some form of notification tooling and locking
//! mechanism provided by `tokio`'s watch
//!
//! the `State` can be cloned and used to share state between threads
//! and services while the `StateSaver` is kept by the management side
//! to eventually save states and allow roll-back in state or resurrection
//! of a service with a given state
//!
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{
    stream::Stream,
    sync::watch::{self, Receiver, Ref, Sender},
};

#[derive(Debug, Clone, Default)]
pub struct NoState;

pub trait State: Default + Clone {}

/// a state wrapper object allowing to maintain a shared state between
/// multiple thread of the same service
pub struct StateHandler<T> {
    inner: Receiver<T>,
    saver: Arc<Sender<T>>,
}

/// a state saver end, it allows to await on state updates and to save
/// the state on updates (if needed).
pub struct StateSaver<T> {
    inner: Receiver<T>,
    saver: Arc<Sender<T>>,
}

impl<T: Clone> StateHandler<T> {
    pub async fn updated(&mut self) -> T {
        self.inner.recv().await.unwrap()
    }
}

impl<T> StateHandler<T> {
    /// borrow the inner state
    ///
    /// however it is better to keep the returned `Ref` short lived as it
    /// blocks any updates of the state
    pub fn state(&self) -> Ref<T> {
        self.inner.borrow()
    }

    /// broadcast a new update of the state to all Clone of `State` and to
    /// all `StateSaver`
    pub fn update(&self, state: T) {
        if self.saver.broadcast(state).is_err() {
            // since we always own at least one instance of receiver
            // there is then no need to worry about not being able
            // to update the state
            unsafe { std::hint::unreachable_unchecked() }
        }
    }
}

impl<T: Clone> StateSaver<T> {
    /// create a new state object
    pub async fn new(initial: T) -> Self {
        let (saver, mut inner) = watch::channel(initial);
        let saver = Arc::new(saver);

        let _ = inner.recv().await;

        Self { inner, saver }
    }

    /// create a state saver from the given state
    pub fn handler(&self) -> StateHandler<T> {
        StateHandler {
            inner: self.inner.clone(),
            saver: Arc::clone(&self.saver),
        }
    }

    /// await on update of the state
    ///
    /// if it returns None, the state is then closed and this state saver
    /// won't receive anymore state updates.
    pub async fn updated(&mut self) -> Option<T> {
        self.inner.recv().await
    }
}

impl<T> Clone for StateHandler<T> {
    fn clone(&self) -> Self {
        StateHandler {
            inner: self.inner.clone(),
            saver: self.saver.clone(),
        }
    }
}

impl<T: Clone> Stream for StateSaver<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<T>> {
        Pin::new(&mut self.get_mut().inner).poll_next(cx)
    }
}

impl State for NoState {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    /// test the initial state is actually the one set
    #[tokio::test]
    async fn llr_ss_1_initial_borrow() {
        const INITIAL: u8 = 42;
        let saver = StateSaver::new(INITIAL).await;
        let state = saver.handler();

        assert_eq!(*state.state(), INITIAL);
    }

    /// test the handler is actually linked to the state
    #[tokio::test]
    async fn llr_ss_2_handler_from_saver() {
        const INITIAL: u8 = 42;
        const UPDATED: u8 = 51;
        let saver = StateSaver::new(INITIAL).await;
        let state = saver.handler();

        state.update(UPDATED);

        assert_eq!(*state.state(), UPDATED);
    }

    /// test the state saver is yield with the initial value
    #[tokio::test]
    async fn llr_ss_3_borrow() {
        const INITIAL: u8 = 42;
        const UPDATED: u8 = 51;
        let mut saver = StateSaver::new(INITIAL).await;
        let state = saver.handler();

        state.update(UPDATED);
        assert_eq!(saver.updated().await, Some(UPDATED));
    }

    /// every time the value is updated by the handler, the call to update
    /// on the state will raise the updated value
    #[tokio::test]
    async fn llr_ss_4_updated_yield_on_update() {
        const INITIAL: u8 = 0;
        const LAST: u8 = 10;
        let mut saver = StateSaver::new(INITIAL).await;
        let state = saver.handler();

        tokio::spawn(async move {
            for i in (INITIAL + 1)..LAST {
                tokio::time::delay_for(Duration::from_millis(20)).await;
                state.update(i);
            }
            tokio::time::delay_for(Duration::from_millis(20)).await;
        });

        for i in (INITIAL + 1)..LAST {
            let value = saver.updated().await.unwrap();

            assert_eq!(value, i);
        }
    }

    /// test that even though the `INITIAL` was not received on `updated`
    /// it is overridden by calls to `update` on the `StateSaver`
    #[tokio::test]
    async fn llr_ss_5_no_initial_yield() {
        const INITIAL: u8 = 42;
        let mut saver = StateSaver::new(INITIAL).await;

        let t = timeout(Duration::from_millis(100), saver.updated()).await;
        assert!(t.is_err());
    }
}
