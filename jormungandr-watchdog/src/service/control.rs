use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::{
    stream::Stream,
    sync::watch::{self, Receiver, Sender},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Control {
    Shutdown,
    Kill,
}

pub struct Controller {
    sender: Sender<Control>,
    receiver: Receiver<Control>,
}

pub struct ControlReader {
    receiver: Receiver<Control>,
}

impl Controller {
    #[allow(clippy::new_without_default)]
    pub async fn new() -> Self {
        let (sender, mut receiver) = watch::channel(Control::Kill);

        let _ = receiver.recv().await;

        Controller { sender, receiver }
    }

    pub fn reader(&self) -> ControlReader {
        ControlReader {
            receiver: self.receiver.clone(),
        }
    }

    pub fn send(&mut self, control: Control) {
        if self.sender.broadcast(control).is_err() {
            // the `Controller` own a Receiver so broadcast
            // cannot fail
            unsafe { std::hint::unreachable_unchecked() }
        }
    }

    pub async fn reset(&mut self) -> ControlReader {
        let mut reader = self.reader();
        self.send(Control::Kill);
        if reader.updated().await.is_none() {
            // `Controller` owns the sender and a send has just ben sent
            // so it is impossible not to have an updated control
            unsafe { std::hint::unreachable_unchecked() }
        }
        reader
    }
}

impl ControlReader {
    pub async fn updated(&mut self) -> Option<Control> {
        self.receiver.recv().await
    }
}

impl Future for ControlReader {
    type Output = Option<Control>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().receiver).poll_next(cx)
    }
}

impl Stream for ControlReader {
    type Item = Control;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.poll(cx)
    }
}
