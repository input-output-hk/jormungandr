//use std::rc;
use std::sync::Arc;

// pub type SharedRef<T> = rc::Rc<T>;
pub type SharedRef<T> = Arc<T>;
