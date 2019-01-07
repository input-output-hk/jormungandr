use super::chain::Hash;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Error {
    BlockNotFound(Hash),
}
