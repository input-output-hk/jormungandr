use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Event {
    pub message: String,
}

impl Event {
    pub fn new<S: Into<String>>(s: S) -> Self {
        Self { message: s.into() }
    }
}

pub trait Observable {
    fn register(self, observer: &Rc<dyn Observer>) -> Self;
    fn notify_all(&self, event: Event);
    fn finish_all(&self);
}

pub trait Observer {
    fn notify(&self, event: Event);
    fn finished(&self);
}
