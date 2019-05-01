#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertError {
    EntryExists,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveError {
    KeyNotFound,
    ValueNotMatching,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateError<T> {
    KeyNotFound,
    ValueCallbackError(T),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplaceError {
    KeyNotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertOrUpdateError<T> {
    Insert(InsertError),
    Update(UpdateError<T>),
}
