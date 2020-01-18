use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::{
    stream::Stream,
    sync::watch::{self, Receiver, Sender},
};

#[derive(Debug, Default, Clone)]
pub struct NoSettings;

pub trait Settings: Default + Clone {}

/// allow managing the settings of a running service
///
/// With this a service setting can be updated and the associated
/// [`SettingsReader`] can be notified of changes.
///
/// [`SettingsReader`]: ./struct.SettingsReader.html
#[derive(Debug)]
pub struct SettingsUpdater<T> {
    sender: Sender<T>,
    receiver: Receiver<T>,
}

/// read the current settings and be notified of any changes in the settings.
#[derive(Debug)]
pub struct SettingsReader<T>(Receiver<T>);

impl<T> SettingsReader<T> {
    /// get a reference to the most updated SettingsReader content
    ///
    /// the reference will hold a `Lock` to the `SettingsUpdater`
    /// blocking any new update. It is recommended to keep the `Ref`
    /// short lived.
    pub fn borrow(&self) -> watch::Ref<T> {
        self.0.borrow()
    }
}

impl<T: Clone> SettingsReader<T> {
    /// on initialization, it will return the initial value, and then will
    /// block until further updates
    ///
    /// return `None` if the `SettingsUpdater` has been closed
    pub async fn updated(&mut self) -> Option<T> {
        self.0.recv().await
    }
}

impl<T> SettingsUpdater<T> {
    /// set the new value of the configuration, awakening the other
    /// end: the Settings. As long as the other end is kept and is
    /// `Settings::updated` value.
    pub fn update(&self, value: T) {
        if self.sender.broadcast(value).is_err() {
            // cannot fail, the receiver is at least owned by the `SettingsReader`
            // attempting to update the settings
            unsafe { std::hint::unreachable_unchecked() }
        }
    }

    pub fn reader(&self) -> SettingsReader<T> {
        SettingsReader(self.receiver.clone())
    }
}

impl<T: Clone> SettingsUpdater<T> {
    pub async fn new(init: T) -> Self {
        let (sender, mut receiver) = watch::channel(init);

        // prevent the initial setting to raise an update event
        let _ = receiver.recv().await;

        SettingsUpdater { sender, receiver }
    }
}

impl<T> Clone for SettingsReader<T> {
    fn clone(&self) -> Self {
        SettingsReader(self.0.clone())
    }
}

impl<T: Clone> Stream for SettingsReader<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<T>> {
        Pin::new(&mut self.get_mut().0).poll_next(cx)
    }
}

impl Settings for NoSettings {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    /// test that the initial value is actually set in the returned Config
    /// object
    #[tokio::test]
    async fn llr_cfg_1_initial_borrow() {
        const INITIAL: u8 = 42;
        let updater = SettingsUpdater::new(INITIAL).await;
        let config = updater.reader();

        assert_eq!(*config.borrow(), INITIAL);
    }

    /// test that both the reader and updater are actually linked
    /// together
    #[tokio::test]
    async fn llr_cfg_2_reader_from_updater() {
        const INITIAL: u8 = 42;
        let updater = SettingsUpdater::new(INITIAL).await;
        let mut config = updater.reader();

        std::mem::drop(updater);

        assert!(config.updated().await.is_none());
    }

    /// updating the setting in the updater updates the value on the
    /// reader too.
    #[tokio::test]
    async fn llr_cfg_3_updating_settings() {
        const INITIAL: u8 = 42;
        const UPDATE: u8 = 51;
        let updater = SettingsUpdater::new(INITIAL).await;
        let config = updater.reader();

        updater.update(UPDATE);

        assert_eq!(*config.borrow(), UPDATE);
    }

    /// every time the value is updated by the updater, the call to update
    /// on the config will raise the updated value
    #[tokio::test]
    async fn llr_cfg_4_updated_yield_on_update() {
        const INITIAL: u8 = 0;
        const LAST: u8 = 10;
        let updater = SettingsUpdater::new(INITIAL).await;
        let mut config = updater.reader();

        tokio::spawn(async move {
            for i in (INITIAL + 1)..LAST {
                tokio::time::delay_for(Duration::from_millis(20)).await;
                updater.update(i);
            }
            tokio::time::delay_for(Duration::from_millis(20)).await;
        });

        for i in (INITIAL + 1)..LAST {
            let value = config.updated().await.unwrap();

            assert_eq!(value, i);
        }
    }

    /// test that the initial value won't yield an event
    #[tokio::test]
    async fn llr_cfg_5_initial_updated() {
        const INITIAL: u8 = 42;
        let updater = SettingsUpdater::new(INITIAL).await;
        let mut config = updater.reader();

        // following config won't return because the initial settings
        // has already been received.
        let t = timeout(Duration::from_millis(20), config.updated()).await;
        assert!(t.is_err());
    }
}
