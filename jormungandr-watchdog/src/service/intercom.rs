use std::ops::{Deref, DerefMut};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct NoIntercom;

pub trait Intercom: 'static {}

pub struct IntercomSender<T>(mpsc::Sender<T>);

pub struct IntercomReceiver<T>(mpsc::Receiver<T>);

impl Intercom for NoIntercom {}

pub fn channel<T: Intercom>() -> (IntercomSender<T>, IntercomReceiver<T>) {
    let (sender, receiver) = mpsc::channel(10);

    (IntercomSender(sender), IntercomReceiver(receiver))
}

impl<T> Clone for IntercomSender<T> {
    fn clone(&self) -> Self {
        IntercomSender(self.0.clone())
    }
}

impl<T> Deref for IntercomSender<T> {
    type Target = mpsc::Sender<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> DerefMut for IntercomSender<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Deref for IntercomReceiver<T> {
    type Target = mpsc::Receiver<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> DerefMut for IntercomReceiver<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
