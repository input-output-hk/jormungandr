/// simple tool to either borrow or owned but without the need
/// to clone the T and to take ownership
pub enum Borrow<'a, T> {
    Borrowed(&'a T),
    Owned(T),
}
impl<'a, T> std::ops::Deref for Borrow<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        match self {
            Borrow::Borrowed(t) => t,
            Borrow::Owned(t) => t,
        }
    }
}
impl<'a, T> From<T> for Borrow<'a, T> {
    fn from(t: T) -> Self {
        Borrow::Owned(t)
    }
}
impl<'a, T> From<&'a T> for Borrow<'a, T> {
    fn from(t: &'a T) -> Self {
        Borrow::Borrowed(t)
    }
}
