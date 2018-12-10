pub trait Block {
    type Hash;
    type Id;

    fn parent_hash(&self) -> &Self::Hash;
    fn slot_id(&self) -> Self::Id;
}