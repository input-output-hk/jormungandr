#[derive(Debug, PartialEq, Eq)]
pub enum InsertError {
    EntryExists,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveError {
    KeyNotFound,
    ValueNotMatching,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateError {
    KeyNotFound,
}
