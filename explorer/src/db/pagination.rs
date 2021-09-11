use sanakirja::{btree, Storable};

use super::{
    chain_storable::{
        BlockId, ChainLength, ExplorerVoteProposal, FragmentId, TransactionInput,
        TransactionOutput, VotePlanId,
    },
    error::ExplorerError,
    pair::Pair,
    Db, SanakirjaTx, SeqNum, P,
};

pub trait PaginationCursor: PartialOrd + Ord + PartialEq + Eq + Clone + Copy {
    const MIN: Self;
    const MAX: Self;
}

impl PaginationCursor for u8 {
    const MIN: Self = u8::MIN;
    const MAX: Self = u8::MAX;
}

impl PaginationCursor for SeqNum {
    const MIN: SeqNum = SeqNum::MIN;
    const MAX: SeqNum = SeqNum::MAX;
}

impl PaginationCursor for ChainLength {
    const MIN: ChainLength = ChainLength::MIN;
    const MAX: ChainLength = ChainLength::MAX;
}

pub trait MapEntry<'a, K, V, C> {
    type Output;

    fn map_entry(&self, _: &'a K, _: &'a V) -> Option<(C, Self::Output)>;
    fn map_cursor(&self, _: C) -> (K, Option<V>);
}

pub struct SanakirjaCursorIter<'a, K, V, C, F>
where
    K: Storable + 'a,
    V: Storable + 'a,
    F: MapEntry<'a, K, V, C>,
{
    txn: &'a SanakirjaTx,
    map_entry: F,
    cursor: btree::Cursor<K, V, P<K, V>>,
    cursor_bounds: Option<(C, C)>,
}

impl<'a, K, V, C, F> Iterator for SanakirjaCursorIter<'a, K, V, C, F>
where
    K: Storable + PartialEq + 'a,
    V: Storable + 'a,
    F: MapEntry<'a, K, V, C>,
{
    type Item = Result<(C, <F as MapEntry<'a, K, V, C>>::Output), ExplorerError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.cursor
            .next(self.txn)
            .map(|item| item.and_then(|(k, v)| self.map_entry.map_entry(k, v)))
            .map_err(ExplorerError::from)
            .transpose()
    }
}

impl<'a, K, V, C, F> DoubleEndedIterator for SanakirjaCursorIter<'a, K, V, C, F>
where
    K: Storable + PartialEq + 'a,
    V: Storable + 'a,
    F: MapEntry<'a, K, V, C>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.cursor
            .prev(self.txn)
            .map(|item| item.and_then(|(k, v)| self.map_entry.map_entry(k, v)))
            .map_err(ExplorerError::from)
            .transpose()
    }
}

impl<'a, K, V, C, F> SanakirjaCursorIter<'a, K, V, C, F>
where
    K: Storable + PartialEq + 'a,
    V: Storable + 'a,
    C: PaginationCursor,
    F: MapEntry<'a, K, V, C>,
{
    /// initialize a new iterator that can be used for cursor based pagination.
    /// `entry_from_cursor` should return the smallest possible entry for the given cursor element,
    /// this is because the internal sanakirja cursor is set at the first position greater than or
    /// equal than what's returned by this function.
    pub fn new(txn: &'a SanakirjaTx, map_entry: F, db: &Db<K, V>) -> Result<Self, ExplorerError> {
        let mut cursor = btree::Cursor::new(txn, db)?;
        let min_entry = map_entry.map_cursor(C::MIN);
        let max_entry = map_entry.map_cursor(C::MAX);

        cursor.set(txn, &min_entry.0, min_entry.1.as_ref())?;

        // TODO: computing the last cursor could be done lazily on demand I guess, but I hope it's
        // not expensive enough to matter, after all is just a single extra lookup. It could also
        // be cached globally, which may be even better, because if we follow relay's graphql
        // specification for connections then I think we always need it.
        let cursor_bounds = cursor
            .current(txn)?
            .and_then(|(k, v)| map_entry.map_entry(k, v))
            .map(|first| -> Result<(C, C), ExplorerError> {
                let (max_key, max_value) = max_entry;
                let mut cursor = btree::Cursor::new(txn, db)?;

                cursor.set(txn, &max_key, max_value.as_ref())?;

                if let Some(last) = cursor.prev(txn)? {
                    if let Some(last) = map_entry.map_entry(last.0, last.1) {
                        Ok((first.0, last.0))
                    } else {
                        // we can unwrap here because we know there is at least one entry before,
                        // because the entry after this was not of this key (otherwise we would be
                        // in the if branch) and we are in the `map` function, so we know there is
                        // at least one entry for the given key.
                        let last = cursor.current(txn)?.unwrap();
                        Ok((first.0, map_entry.map_entry(last.0, last.1).unwrap().0))
                    }
                } else {
                    Ok((first.0, first.0))
                }
            })
            .transpose()?;

        Ok(Self {
            txn,
            map_entry,
            cursor,
            cursor_bounds,
        })
    }

    /// this returns None only when the iterator is empty
    pub fn first_cursor(&self) -> Option<&C> {
        self.cursor_bounds.as_ref().map(|(first, _last)| first)
    }

    /// this returns None only when the iterator is empty
    pub fn last_cursor(&self) -> Option<&C> {
        self.cursor_bounds.as_ref().map(|(_first, last)| last)
    }

    /// put the initial iterator position to `cursor`. This has no effect if the cursor is outside
    /// the bounds, and will return Ok(false) if that's the case.
    pub fn seek(&mut self, cursor: C) -> Result<bool, ExplorerError> {
        match self.cursor_bounds {
            Some((a, b)) if cursor >= a && cursor <= b => {
                let (key, value) = self.map_entry.map_cursor(cursor);
                self.cursor.set(self.txn, &key, value.as_ref())?;

                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub fn seek_end(&mut self) -> Result<(), ExplorerError> {
        if let Some((_, last)) = self.cursor_bounds {
            assert!(self.seek(last)?);
        }

        Ok(())
    }
}

pub type TxsByAddress<'a> =
    SanakirjaCursorIter<'a, SeqNum, Pair<SeqNum, FragmentId>, SeqNum, AddressId>;
pub type BlocksInBranch<'a> = SanakirjaCursorIter<'a, ChainLength, BlockId, ChainLength, ()>;
pub type FragmentInputIter<'a> = SanakirjaCursorIter<
    'a,
    Pair<FragmentId, u8>,
    TransactionInput,
    u8,
    FragmentContentId<TransactionInput>,
>;
pub type FragmentOutputIter<'a> = SanakirjaCursorIter<
    'a,
    Pair<FragmentId, u8>,
    TransactionOutput,
    u8,
    FragmentContentId<TransactionOutput>,
>;
pub type BlockFragmentsIter<'a> =
    SanakirjaCursorIter<'a, BlockId, Pair<u8, FragmentId>, u8, BlockContentsId>;
pub type VotePlanProposalsIter<'a> =
    SanakirjaCursorIter<'a, Pair<VotePlanId, u8>, ExplorerVoteProposal, u8, VotePlanProposalsId>;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AddressId(SeqNum);

impl From<&SeqNum> for AddressId {
    fn from(i: &SeqNum) -> Self {
        Self(*i)
    }
}

impl<'a> MapEntry<'a, SeqNum, Pair<SeqNum, FragmentId>, SeqNum> for AddressId {
    type Output = &'a FragmentId;

    fn map_entry(
        &self,
        k: &'a SeqNum,
        v: &'a Pair<SeqNum, FragmentId>,
    ) -> Option<(SeqNum, Self::Output)> {
        if k == &self.0 {
            Some((v.a, &v.b))
        } else {
            None
        }
    }

    fn map_cursor(&self, cursor: SeqNum) -> (SeqNum, Option<Pair<SeqNum, FragmentId>>) {
        (
            self.0,
            Some(Pair {
                a: cursor,
                b: FragmentId::MIN,
            }),
        )
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FragmentContentId<V>(FragmentId, std::marker::PhantomData<V>);

impl<V> From<&FragmentId> for FragmentContentId<V> {
    fn from(i: &FragmentId) -> Self {
        Self(i.clone(), std::marker::PhantomData)
    }
}

impl<V> AsRef<FragmentId> for FragmentContentId<V> {
    fn as_ref(&self) -> &FragmentId {
        &self.0
    }
}

impl<'a, V: 'a> MapEntry<'a, Pair<FragmentId, u8>, V, u8> for FragmentContentId<V> {
    type Output = &'a V;

    fn map_entry(&self, k: &'a Pair<FragmentId, u8>, v: &'a V) -> Option<(u8, Self::Output)> {
        if &k.a == self.as_ref() {
            Some((k.b, &v))
        } else {
            None
        }
    }

    fn map_cursor(&self, cursor: u8) -> (Pair<FragmentId, u8>, Option<V>) {
        (
            Pair {
                a: self.as_ref().clone(),
                b: cursor,
            },
            None,
        )
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockContentsId(BlockId);

impl From<&BlockId> for BlockContentsId {
    fn from(i: &BlockId) -> Self {
        Self(i.clone())
    }
}

impl<'a> MapEntry<'a, BlockId, Pair<u8, FragmentId>, u8> for BlockContentsId {
    type Output = &'a FragmentId;

    fn map_entry(&self, k: &'a BlockId, v: &'a Pair<u8, FragmentId>) -> Option<(u8, Self::Output)> {
        if k == &self.0 {
            Some((v.a, &v.b))
        } else {
            None
        }
    }

    fn map_cursor(&self, cursor: u8) -> (BlockId, Option<Pair<u8, FragmentId>>) {
        (
            self.0.clone(),
            Some(Pair {
                a: cursor,
                b: FragmentId::MIN,
            }),
        )
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VotePlanProposalsId(VotePlanId);

impl From<&VotePlanId> for VotePlanProposalsId {
    fn from(i: &VotePlanId) -> Self {
        Self(i.clone())
    }
}

impl<'a> MapEntry<'a, Pair<VotePlanId, u8>, ExplorerVoteProposal, u8> for VotePlanProposalsId {
    type Output = &'a ExplorerVoteProposal;

    fn map_entry(
        &self,
        k: &'a Pair<VotePlanId, u8>,
        v: &'a ExplorerVoteProposal,
    ) -> Option<(u8, Self::Output)> {
        if k.a == self.0 {
            Some((k.b, v))
        } else {
            None
        }
    }

    fn map_cursor(&self, cursor: u8) -> (Pair<VotePlanId, u8>, Option<ExplorerVoteProposal>) {
        (
            Pair {
                a: self.0.clone(),
                b: cursor,
            },
            None,
        )
    }
}

impl<'a, K, V> MapEntry<'a, K, V, K> for ()
where
    V: 'a,
    K: 'a + Clone,
{
    type Output = &'a V;

    fn map_entry(&self, k: &'a K, v: &'a V) -> Option<(K, Self::Output)> {
        Some((k.clone(), v))
    }

    fn map_cursor(&self, k: K) -> (K, Option<V>) {
        (k, None)
    }
}
