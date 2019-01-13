//! chain core properties
//!
//! define the different properties a _supported_ chain needs to
//! implement to work in our models.
//!
//! # Block
//!
//! The Block is the atomic element that compose a chain. Or in other
//! words the chain is composed of a succession of `Block`.
//!
//! the `Block` trait implements the necessary feature we expect of
//! a `Block` in the chain. Having a function that requires the object
//! to implement the Block traits means that we are expecting to have
//! only access to:
//!
//! * the block and its parent's identifier (the block hash);
//! * the block number, its position in the blockchain relative
//!   to the beginning of the chain. We often call this number
//!   the block Date.
//!
//! # HasTransaction and Transaction
//!
//! These traits are mainly fit for the purpose of the Unspent Transaction
//! Output (UTxO) model.
//!
//! # Ledger
//!
//! this trait is to make sure we are following the Transactions of the chain
//! appropriately.
//!
//! # LeaderSelection
//!
//! This trait is following the protocol of the blockchain is followed
//! properly and determined a given instance of the LeaderSelection object
//! is selected to write a block in the chain.
//!

use std::hash::Hash;

/// Trait identifying the block identifier type.
pub trait BlockId: Eq + Hash {}

/// Trait identifying the block date type.
pub trait BlockDate: Eq + Ord {}

/// Trait identifying the transaction identifier type.
pub trait TransactionId: Eq + Hash {}

/// Trait identifying the block header type.
pub trait Header {}

impl Header for () {}

/// Block property
///
/// a block is part of a chain of block called Blockchain.
/// the chaining is done via one block pointing to another block,
/// the parent block (the previous block).
///
/// This means that a blockchain is a link-list, ordered from the most
/// recent block to the furthest/oldest block.
///
/// The Oldest block is called the Genesis Block.
pub trait Block: Serialize + Deserialize {
    /// the Block identifier. It must be unique. This mean that
    /// 2 different blocks have 2 different identifiers.
    ///
    /// In bitcoin this block is a SHA2 256bits. For Cardano's
    /// blockchain it is Blake2b 256bits.
    type Id: BlockId;

    /// the block date (also known as a block number) represents the
    /// absolute position of the block in the chain. This can be used
    /// for random access (if the storage algorithm allows it) or for
    /// identifying the position of a block in a given epoch or era.
    type Date: BlockDate;

    /// The block header. If provided by the blockchain, the header
    /// can be used to transmit block's metadata via a network protocol
    /// or in other uses where the full content of the block is not desirable.
    /// An implementation that does not feature headers can use the unit
    /// type `()`.
    type Header: Header;

    /// return the Block's identifier.
    fn id(&self) -> Self::Id;

    /// get the parent block identifier (the previous block in the
    /// blockchain).
    fn parent_id(&self) -> Self::Id;

    /// get the block date of the block
    fn date(&self) -> Self::Date;

    /// Gets the block's header.
    fn header(&self) -> Self::Header;
}

/// define a transaction within the blockchain. This transaction can be used
/// for the UTxO model. However it can also be used for any other elements that
/// the blockchain has (a transaction type to add Stacking Pools and so on...).
///
pub trait Transaction: Serialize + Deserialize {
    /// the input type of the transaction (if none use `()`).
    type Input;
    /// the output type of the transaction (if none use `()`).
    type Output;
    /// a unique identifier of the transaction. For 2 different transactions
    /// we must have 2 different `Id` values.
    type Id: TransactionId;

    fn inputs<'a>(&'a self) -> std::slice::Iter<'a, Self::Input>;
    fn outputs<'a>(&'a self) -> std::slice::Iter<'a, Self::Output>;

    /// return the Transaction's identifier.
    fn id(&self) -> Self::Id;
}

/// accessor to transactions within a block
///
/// This trait is generic enough to show there is multiple types
/// of transaction possibles:
///
/// * UTxO
/// * certificate registrations
/// * ...
pub trait HasTransaction<T: Transaction> {
    /// returns an iterator over the Transactions
    fn transactions<'a>(&'a self) -> std::slice::Iter<'a, T>;
}

/// Updates type needs to implement this feature so we can easily
/// compose the Updates objects.
///
pub trait Update {
    /// allowing to build unions of updates will allow us to compress
    /// atomic modifications.
    ///
    /// For example, in the cardano model we can consider compressing
    /// the Update diff of all the EPOCHs below `EPOCH - 2`
    ///
    fn union(self, other: Self) -> Self;

    /// inverse an update. This will be useful for Rollback in case the
    /// node has decided to rollback to a previous fork and un apply the
    /// given update.
    fn inverse(self) -> Self;

    fn empty() -> Self;
}

/// Define the Ledger side of the blockchain. This is not really on the blockchain
/// but should be able to maintain a valid state of the overall blockchain at a given
/// `Block`.
pub trait Ledger<T: Transaction> {
    /// a Ledger Update. An atomic representation of a set of changes
    /// into the ledger's state.
    ///
    /// This can be seen like a git Diff where we can see what is going
    /// to be removed from the Ledger state and what is going to be added.
    type Update: Update;

    /// Ledger's errors
    type Error: std::error::Error;

    /// check the input exists in the given ledger state
    ///
    /// i.e. in the UTxO model the Input will be something like the Transaction's Id
    /// and the index of the output within the output array.
    /// If the Output is not present it is possible that it does not exist or has
    /// already been spent in another transaction.
    fn input<'a>(&'a self, input: &T::Input) -> Result<&'a T::Output, Self::Error>;

    /// create a new Update from the given transaction.
    fn diff_transaction(&self, transaction: &T) -> Result<Self::Update, Self::Error>;

    /// create a combined Update from the given transactions
    ///
    fn diff<'a, I>(&self, transactions: I) -> Result<Self::Update, Self::Error>
    where
        I: IntoIterator<Item = &'a T> + Sized,
        T: 'a,
    {
        let mut update = Self::Update::empty();

        for transaction in transactions {
            update = self.diff_transaction(transaction)?;
        }

        Ok(update)
    }

    /// apply an update to the leger.
    fn apply(&mut self, update: Self::Update) -> Result<&mut Self, Self::Error>;
}

/// interface for the leader selection algorithm
///
/// this is the interface that is responsible to verify the Block are
/// created by the right Leaders (i.e. that everyone follows the
/// consensus algorithm).
///
/// This is also the same interface that is used to detect if we are the
/// leader for the block at the given date.
pub trait LeaderSelection {
    /// a leader selection Update. This is an atomic representation of
    /// the set of changes to apply to the leader selection state.
    ///
    /// Having an atomic representation of the changes allow other
    /// interesting properties:
    ///
    /// * generic testing;
    /// * diff based storage;
    ///
    type Update;

    /// the block that we will get the information from
    type Block: Block;

    /// Leader Selection error type
    type Error: std::error::Error;

    /// given a Block, create an Update diff to see what are the changes
    /// that will come with this new block.
    ///
    /// This function is also responsible to check the validity of the block
    /// within the blockchain but not to check the Transactional entities.
    /// The transaction part are verified with the [`Transaction::diff`]
    /// method.
    ///
    /// Here we want to check the validity of the consensus and of the block
    /// signature.
    fn diff(&self, input: &Self::Block) -> Result<Self::Update, Self::Error>;

    /// apply the Update to the LeaderSelection
    fn apply(&mut self, update: Self::Update) -> Result<(), Self::Error>;

    /// return if this instance of the LeaderSelection is leader of the
    /// blockchain at the given date.
    ///
    fn is_leader_at(&self, date: <Self::Block as Block>::Date) -> Result<bool, Self::Error>;
}

/// the settings of the blockchain this is something that can be used to maintain
/// the blockchain protocol update details:
///
pub trait Settings {
    type Update;
    type Block: Block;
    type Error: std::error::Error;

    /// read the block update settings and see if we need to store
    /// updates. Protocols may propose vote mechanism, this Update
    /// and the settings need to keep track of these here.
    fn diff(&self, input: &Self::Block) -> Result<Self::Update, Self::Error>;

    /// apply the Update to the LeaderSelection
    fn apply(&mut self, update: Self::Update) -> Result<(), Self::Error>;
}

/// Define that an object can be written to a `Write` object.
pub trait Serialize {
    type Error: std::error::Error + From<std::io::Error>;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error>;
}

/// Define that an object can be read from a `Read` object.
pub trait Deserialize: Sized {
    type Error: std::error::Error + From<std::io::Error>;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error>;
}

impl<T: Serialize> Serialize for &T {
    type Error = T::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), T::Error> {
        (**self).serialize(writer)
    }
}

#[cfg(feature = "property-test-api")]
pub mod testing {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};

    /// test that any arbitrary given object can serialize and deserialize
    /// back into itself (i.e. it is a bijection,  or a one to one match
    /// between the serialized bytes and the object)
    pub fn serialization_bijection<T>(t: T) -> bool
    where
        T: Arbitrary + Serialize + Deserialize + Eq,
    {
        let mut vec = Vec::new();
        t.serialize(&mut vec).unwrap();
        let decoded_t = T::deserialize(&mut &vec[..]).unwrap();
        decoded_t == t
    }

    /// test that arbitrary generated transaction fails, this test requires
    /// that all the objects inside the transaction are arbitrary generated.
    /// There is a very small probability of the event that all the objects
    /// will match, i.e. the contents of the transaction list of the subscribers
    /// and signatures will compose into a valid transaction, but if such
    /// event would happen it can be treated as error due to lack of the
    /// randomness.
    pub fn prop_bad_transaction_fails<L, T>(ledger: L, transaction: T) -> TestResult
    where
        L: Ledger<T> + Arbitrary,
        T: Transaction + Arbitrary,
    {
        if transaction.inputs().len() == 0 && transaction.outputs().len() == 0 {
            return TestResult::discard();
        }
        TestResult::from_bool(ledger.diff_transaction(&transaction).is_err())
    }

    /// Pair with a ledger and transaction that is valid in such state.
    /// This structure is used for tests generation, when the framework
    /// require user to pass valid transaction.
    #[derive(Clone, Debug)]
    pub struct LedgerWithValidTransaction<L, T>(pub L, pub T);

    /// Test that checks if arbitrary valid transaction succeed and can
    /// be added to the ledger.
    pub fn prop_good_transactions_succeed<L, T>(
        input: &mut LedgerWithValidTransaction<L, T>,
    ) -> bool
    where
        L: Ledger<T> + Arbitrary,
        T: Transaction + Arbitrary,
    {
        match input.0.diff_transaction(&input.1) {
            Err(e) => panic!("error {:#?}", e),
            Ok(diff) => input.0.apply(diff).is_ok(),
        }
    }

    /// Trait that provides a property of generation valid transactions
    /// from the current state.
    pub trait GenerateTransaction<T: Transaction> {
        fn generate_transaction<G>(&mut self, g: &mut G) -> T
        where
            G: Gen;
    }

    /// Generate a number of transactions and run them, it's not
    /// expected to have any errors during the run.
    pub fn run_valid_transactions<'a, G, L, T>(g: &'a mut G, ledger: &'a mut L, n: usize) -> ()
    where
        G: Gen,
        L: Ledger<T> + GenerateTransaction<T>,
        T: Transaction,
    {
        for _ in 0..n {
            let tx = ledger.generate_transaction(g);
            let update = ledger.diff_transaction(&tx).unwrap();
            ledger.apply(update).unwrap();
        }
    }

    /// Checks that transaction id uniquely identifies the transaction,
    /// i.e.
    ///
    /// ```text
    /// forall tx1, tx2:Transaction: tx1.id() == tx2.id() <=> tx1 == tx2
    /// ```
    pub fn transaction_id_is_unique<T>(tx1: T, tx2: T) -> bool
    where
        T: Transaction + Arbitrary + PartialEq,
        T::Id: PartialEq,
    {
        let id1 = tx1.id();
        let id2 = tx2.id();
        (id1 == id2 && tx1 == tx2) || (id1 != id2 && tx1 != tx2)
    }

    /// Checks the associativity
    /// i.e.
    ///
    /// ```text
    /// forall u : Update, v: Update, w:Update . u.union(v.union(w))== (u.union(v)).union(w)
    /// ```
    pub fn update_associativity<U>(u: U, v: U, w: U) -> bool
    where
        U: Update + Arbitrary + PartialEq + Clone,
    {
        let result1 = u.clone().union(v.clone().union(w.clone()));
        let result2 = (u.clone().union(v.clone())).union(w.clone());
        result1 == result2
    }

    /// Checks the identify element
    /// i.e.
    ///
    /// ```text
    /// forall u : Update . u.union(empty)== u
    /// ```
    pub fn update_identity_element<U>(update: U) -> bool
    where
        U: Update + Arbitrary + PartialEq + Clone,
    {
        let result = update.clone().union(U::empty());
        result == update
    }

    /// Checks for the inverse element
    /// i.e.
    ///
    /// ```text
    /// forall u : Update . u.inverse().union(u) == empty
    /// ```
    pub fn update_inverse_element<U>(update: U) -> bool
    where
        U: Update + Arbitrary + PartialEq + Clone,
    {
        let inversed = update.clone().inverse();
        inversed.union(update) == U::empty()
    }

    /// Checks the commutativity of the Union
    /// i.e.
    ///
    /// ```text
    /// forall u : Update, v: Update . u.union(v)== v.union(u)
    /// ```
    pub fn update_union_commutative<U>(u1: U, u2: U) -> bool
    where
        U: Update + Arbitrary + PartialEq + Clone,
    {
        let r1 = u1.clone().union(u2.clone());
        let r2 = u2.clone().union(u1.clone());
        r1 == r2
    }
}
