use sled::Tree;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use thiserror::Error;

/// the root domain of the settings.
#[derive(Clone)]
pub struct Settings {
    inner: Domain,
}

/// The settings domain. This object allows keeping track of the
/// encapsulation of the settings parameters and also to keep
/// separated the different operations on different domains
///
/// For example, the network module will only be interested about
/// the network settings.
#[derive(Clone)]
pub struct Domain {
    inner: Tree,
    domain: String,
}

/// Subscriber for events that occurs in within the associated Domain.
///
/// Implements both synchronous subscriber (`Iterator`) and asynchronous
/// subscriber (`Future`).
pub struct Subscriber(sled::Subscriber);

#[derive(Debug, Clone)]
pub struct Event;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{source}")]
    Storage {
        #[from]
        source: sled::Error,
    },
}

impl Domain {
    fn new<S>(inner: Tree, domain: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            inner,
            domain: domain.into(),
        }
    }

    fn key<K>(&self, key: K) -> String
    where
        K: std::fmt::Display,
    {
        format!("{}.{}", self.domain(), key)
    }

    /// Get the domain full name
    pub fn domain(&self) -> &str {
        self.domain.as_str()
    }

    /// create a subdomain off the given domain
    ///
    /// # panics
    ///
    /// This function will panic if the given domain is empty
    ///
    pub fn sub_domain<S>(&self, domain: S) -> Self
    where
        S: AsRef<str>,
    {
        assert!(!domain.as_ref().is_empty(), "Domain cannot be empty");
        Self::new(self.inner.clone(), self.key(domain.as_ref()))
    }

    /// get the value associated to the given key (if any) within the
    /// current Domain.
    ///
    pub fn get<K>(&self, key: K) -> Result<Option<String>, Error>
    where
        K: std::fmt::Display,
    {
        let key = self.key(key);
        let res = self.inner.get(key)?;

        if let Some(res) = res {
            let prev = String::from_utf8_lossy(res.as_ref()).into_owned();
            Ok(Some(prev))
        } else {
            Ok(None)
        }
    }

    /// insert a new key/value or replace the existing one with the new value.
    ///
    /// If it was a replace, the previous value is returned. Otherwise new value
    /// will return `None`.
    pub fn insert<K, V>(&self, key: K, value: V) -> Result<Option<String>, Error>
    where
        K: std::fmt::Display,
        V: AsRef<str>,
    {
        let key = self.key(key);
        let res = self.inner.insert(key, value.as_ref())?;

        if let Some(prev) = res {
            let prev = String::from_utf8_lossy(prev.as_ref()).into_owned();
            Ok(Some(prev))
        } else {
            Ok(None)
        }
    }

    /// subscribe to changes in this domain
    ///
    /// any changes in this domain or any of its subdomain will
    /// raise an Event.
    pub fn subscribe(&self) -> Subscriber {
        Subscriber(self.inner.watch_prefix(&self.domain))
    }
}

impl Settings {
    /// create a new settings in within the given `sled`'s Tree.
    pub fn new(inner: Tree) -> Self {
        Self {
            inner: Domain::new(inner, String::new()),
        }
    }

    /// create a settings domain
    ///
    /// # panics
    ///
    /// This function will panic if the domain name is empty
    pub fn domain<D>(&self, domain: D) -> Domain
    where
        D: AsRef<str>,
    {
        assert!(!domain.as_ref().is_empty(), "Domain cannot be empty");
        self.inner.sub_domain(domain)
    }
}

impl Iterator for Subscriber {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|_| Event)
    }
}

impl Future for Subscriber {
    type Output = Option<Event>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let pinned = Pin::new(&mut self.get_mut().0);

        match Future::poll(pinned, cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(_)) => Poll::Ready(Some(Event)),
        }
    }
}
