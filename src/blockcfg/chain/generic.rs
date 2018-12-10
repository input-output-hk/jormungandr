/// this defines the Chain part of a Block. A Block must always
/// have (in our model) a parent's Hash and an Identifier.
///
/// The difference between the Block's Hash and its Id is that
/// the first one identifies the block within all the blocks of
/// the chain while the second one gives us information about
/// when the Block was created. It is possible that there is
/// multiple Blocks with the same Id (this is called a fork) but
/// it is _assumed_ impossible for 2 different Blocks to have
/// the same Hash.
///
pub trait Block {
    /// define what is the type of the Hash to use to identify
    /// a Block in the Chain.
    type Hash;

    /// define what identify the slot of the Block within the chain
    type Id;

    /// the Hash of the parent of this block within the Chain
    fn parent_hash(&self) -> &Self::Hash;
    /// the slot identifier that identify where the block lies within
    /// the chain.
    fn slot_id(&self) -> Self::Id;
}