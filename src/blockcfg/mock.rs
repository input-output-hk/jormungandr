//! This module defines some basic type to try to mock the blockchain
//! and be able to run simpler tests.
//!

use std::collections::HashMap;

use crate::blockcfg::{property, serialization};

use bincode;
use cardano::hash;
use cardano::hdwallet as crypto;

/// Non unique identifier of the transaction position in the
/// blockchain. There may be many transactions related to the same
/// `SlotId`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct SlotId(u32, u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Hash(hash::Blake2b256);
impl Hash {
    pub fn hash_bytes(bytes: &[u8]) -> Self {
        Hash(hash::Blake2b256::new(bytes))
    }
}
impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/// TODO: this public key contains the chain code in it too
/// during serialisation this might not be needed
/// removing it will save 32bytes of non necessary storage (github #93)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct PublicKey(crypto::XPub);
impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
impl PublicKey {
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        self.0.verify(message, &signature.0)
    }
}

/// Private key of the entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateKey(crypto::XPrv);
impl AsRef<[u8]> for PrivateKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
impl PrivateKey {
    pub fn public(&self) -> PublicKey {
        PublicKey(self.0.public())
    }
    pub fn sign(&self, data: &[u8]) -> Signature {
        Signature(self.0.sign(data))
    }
}

/// Cryptographic signature.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Signature(crypto::Signature<()>);
impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/// Unspent transaction value.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Value(u64);

/// Address. Currently address is just a hash of
/// the public key.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Address(Hash);
impl Address {
    pub fn new(public_key: &PublicKey) -> Self {
        Address(Hash::hash_bytes(public_key.as_ref()))
    }
}
impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/// Unspent transaction pointer.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct UtxoPointer {
    /// Id of the transaction there UT was created.
    pub transaction_id: TransactionId,
    /// Index of the output (wallet) that UT represents.
    pub output_index: u32,
}
impl UtxoPointer {
    pub fn new(transaction_id: TransactionId, output_index: u32) -> Self {
        UtxoPointer {
            transaction_id,
            output_index,
        }
    }
}

/// Structure that proofs that certain user agrees with
/// some data. This structure is used to sign `Transaction`
/// and get `SignedTransaction` out.
///
/// It's important that witness works with opaque structures
/// and may not know the contents of the internal transaction.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Witness {
    pub signature: Signature,
    pub public_key: PublicKey,
}

impl Witness {
    /// Creates new `Witness` value.
    pub fn new(transaction_id: TransactionId, private_key: &PrivateKey) -> Self {
        let sig = private_key.sign(transaction_id.as_ref());
        Witness {
            signature: sig,
            public_key: private_key.public(),
        }
    }

    /// Checks if a witness emitter matches the `Output` address.
    ///
    /// This check is needed because each Utxo in the transaction
    /// must be signed by the wallet holder.
    pub fn matches(&self, output: &Output) -> bool {
        let addr = Address::new(&self.public_key);
        addr == output.0
    }

    /// Verify the given `TransactionId` using the witness.
    pub fn verifies(&self, transaction_id: TransactionId) -> bool {
        self.public_key
            .verify(transaction_id.as_ref(), &self.signature)
    }
}

/// Information how tokens are spent.
/// A value of tokens is sent to the address.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Output(pub Address, pub Value);

/// Id of the transaction.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct TransactionId(Hash);
impl AsRef<[u8]> for TransactionId {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/// Transaction, transaction maps old unspent tokens into the
/// set of the new addresses.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Transaction {
    pub inputs: Vec<UtxoPointer>,
    pub outputs: Vec<Output>,
}

/// Each transaction must be signed in order to be executed
/// by the ledger. `SignedTransaction` represents such a transaction.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SignedTransaction {
    pub tx: Transaction,
    pub witnesses: Vec<Witness>,
}

/// `Block` is an element of the blockchain it contains multiple
/// transaction and a reference to the parent block. Alongside
/// with the position of that block in the chain.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Block {
    pub slot_id: SlotId,
    pub parent_hash: Hash,

    pub transactions: Vec<SignedTransaction>,
}

impl serialization::Deserialize for Block {
    // FIXME: decide on appropriate format for mock blockchain

    type Error = bincode::Error;

    fn deserialize(data: &[u8]) -> Result<Block, bincode::Error> {
        bincode::deserialize(data)
    }
}

impl property::Block for Block {
    type Id = Hash;
    type Date = SlotId;

    fn id(&self) -> Self::Id {
        let bytes = bincode::serialize(self).expect("unable to serialize block");
        Hash::hash_bytes(&bytes)
    }
    fn parent_id(&self) -> &Self::Id {
        &self.parent_hash
    }
    fn date(&self) -> Self::Date {
        self.slot_id
    }
}
impl property::HasTransaction for Block {
    type Transaction = SignedTransaction;

    fn transactions<'a>(&'a self) -> std::slice::Iter<'a, Self::Transaction> {
        self.transactions.iter()
    }
}

impl property::Transaction for Transaction {
    type Input = UtxoPointer;
    type Output = Output;
    type Id = TransactionId;
    fn id(&self) -> Self::Id {
        let bytes = bincode::serialize(self).expect("unable to serialize transaction");
        TransactionId(Hash::hash_bytes(&bytes))
    }
}

impl property::Transaction for SignedTransaction {
    type Input = UtxoPointer;
    type Output = Output;
    type Id = TransactionId;
    fn id(&self) -> Self::Id {
        self.tx.id()
    }
}

#[derive(Debug, Clone)]
pub struct Ledger {
    unspent_outputs: HashMap<UtxoPointer, Output>,
}
impl Ledger {
    /// Generate new ledges with an empty state.
    pub fn new(genesis: HashMap<UtxoPointer, Output>) -> Self {
        Ledger {
            unspent_outputs: genesis,
        }
    }

}

#[derive(Debug, Clone)]
pub struct Diff {
    spent_outputs: HashMap<UtxoPointer, Output>,
    new_unspent_outputs: HashMap<UtxoPointer, Output>,
}
impl Diff {
    fn new() -> Self {
        Diff {
            spent_outputs: HashMap::new(),
            new_unspent_outputs: HashMap::new(),
        }
    }

    fn extend(&mut self, other: Self) {
        self.new_unspent_outputs.extend(other.new_unspent_outputs);
        self.spent_outputs.extend(other.spent_outputs);
    }
}

#[derive(Debug, Clone)]
pub enum Error {
    /// If the Ledger could not find the given input in the UTxO list it will
    /// report this error.
    InputDoesNotResolve(UtxoPointer),

    /// if the Ledger finds that the input has already been used once in a given
    /// transaction or block of transactions it will report this error.
    ///
    /// the input here is the given input used twice,
    /// the output here is the output set in the first occurrence of the input, it
    /// will provide a bit of information to the user to figure out what went wrong
    DoubleSpend(UtxoPointer, Output),

    /// This error will happen if the input was already set and is now replaced
    /// by another output.
    ///
    /// I.E: the value output has changed but the input is the same. This should not
    /// happen since changing the output will change the transaction identifier
    /// associated to this output.
    ///
    /// first the input in common, then the original output and finally the new output
    InputWasAlreadySet(UtxoPointer, Output, Output),

    /// error occurs if the signature is invalid: either does not match the initial output
    /// or it is not cryptographically valid.
    InvalidSignature(UtxoPointer, Output, Witness),

    /// error occurs when one of the witness does not sing entire
    /// transaction properly.
    InvalidTxSignature(Witness),

    /// Transaction summ is not equal to zero, this means that there
    /// were some money taken out of the nowhere, or some money that
    /// had dissapeared.
    TransactionSumIsNonZero(u64, u64),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::InputDoesNotResolve(_) => write!(f, "Input does not resolve to an UTxO"),
            Error::DoubleSpend(_, _) => write!(f, "UTxO spent twice in the same transaction"),
            Error::InputWasAlreadySet(_, _, _) => {
                write!(f, "Input was already present in the Ledger")
            }
            Error::InvalidSignature(_, _, _) => write!(f, "Input is not signed properly"),
            Error::InvalidTxSignature(_) => write!(f, "Transaction was not signed"),
            Error::TransactionSumIsNonZero(_, _) => write!(f, "Transaction is not zero"),
        }
    }
}
impl std::error::Error for Error {}

impl property::Ledger for Ledger {
    type Transaction = SignedTransaction;
    type Diff = Diff;
    type Error = Error;

    /// Create and a diff based on the transaction. The
    /// transaction is validated. In case if validation
    /// fails corresponding error will be returned.
    fn diff_transaction(&self, transaction: &Self::Transaction) -> Result<Self::Diff, Self::Error> {
        use crate::blockcfg::property::Transaction;

        let mut diff = Diff::new();
        let id = transaction.id();
        // 1. validate transaction without looking into the context
        // and that each input is validated by the matching key.
        for (input, witness) in transaction
            .tx
            .inputs
            .iter()
            .zip(transaction.witnesses.iter())
        {
            if !witness.verifies(transaction.tx.id()) {
                return Err(Error::InvalidTxSignature(witness.clone()));
            }
            if let Some(output) = self.unspent_outputs.get(&input) {
                if !witness.matches(&output) {
                    return Err(Error::InvalidSignature(*input, *output, witness.clone()));
                }
                if let Some(output) = diff.spent_outputs.insert(*input, *output) {
                    return Err(Error::DoubleSpend(*input, output));
                }
            } else {
                return Err(Error::InputDoesNotResolve(*input));
            }
        }
        // 2. prepare to add the new outputs
        for (index, output) in transaction.tx.outputs.iter().enumerate() {
            diff.new_unspent_outputs
                .insert(UtxoPointer::new(id, index as u32), *output);
        }
        // 3. verify that transaction sum is zero.
        let spent = diff
            .spent_outputs
            .iter()
            .fold(0, |acc, (_, Output(_, Value(x)))| acc + x);
        let new_unspent = diff
            .new_unspent_outputs
            .iter()
            .fold(0, |acc, (_, Output(_, Value(x)))| acc + x);
        if spent != new_unspent {
            return Err(Error::TransactionSumIsNonZero(spent, new_unspent));
        }

        Ok(diff)
    }

    /// Compose a single diff into a larger diff.
    fn diff<'a, I>(&self, transactions: I) -> Result<Self::Diff, Self::Error>
    where
        I: Iterator<Item = &'a Self::Transaction> + Sized,
        Self::Transaction: 'a,
    {
        let mut diff = Diff::new();

        for transaction in transactions {
            diff.extend(self.diff_transaction(transaction)?);
        }

        Ok(diff)
    }

    /// Apply the diff.
    fn add(&mut self, diff: Self::Diff) -> Result<&mut Self, Self::Error> {
        for spent_output in diff.spent_outputs.keys() {
            if let None = self.unspent_outputs.remove(spent_output) {
                return Err(Error::InputDoesNotResolve(*spent_output));
            }
        }

        for (input, output) in diff.new_unspent_outputs {
            if let Some(original_output) = self.unspent_outputs.insert(input, output) {
                return Err(Error::InputWasAlreadySet(input, original_output, output));
            }
        }

        Ok(self)
    }
}

#[cfg(test)]
mod ledger {

    use super::*;
    use quickcheck::{Arbitrary, StdGen};
    use rand::prelude::*;

    /// Helper structure that keeps information about
    /// the users it can be used for a simple generation of
    /// new keys.
    struct Environment {
        gen: StdGen<rand::ThreadRng>,
        users: HashMap<usize, PrivateKey>,
        keys: HashMap<Address, PrivateKey>,
    }

    impl Environment {
        pub fn new() -> Self {
            let g = StdGen::new(thread_rng(), 10);
            Environment {
                gen: g,
                users: HashMap::new(),
                keys: HashMap::new(),
            }
        }

        pub fn genesis(&mut self, users: HashMap<usize, u64>) -> (Transaction, Ledger) {
            use blockcfg::mock;
            use blockcfg::property::Transaction;

            let genesis = mock::Transaction {
                inputs: Vec::new(),
                outputs: users
                    .iter()
                    .map(|(key, &u)| Output(Address::new(&self.user(*key).public()), Value(u)))
                    .collect(),
            };

            let initial_state: HashMap<UtxoPointer, Output> = users
                .iter()
                .enumerate()
                .map(|(idx, (i, &u))| {
                    (
                        UtxoPointer {
                            transaction_id: genesis.id(),
                            output_index: idx as u32,
                        },
                        Output(Address::new(&self.user(*i).public()), Value(u)),
                    )
                })
                .collect();

            (genesis, Ledger::new(initial_state))
        }

        pub fn user(&mut self, id: usize) -> PrivateKey {
            let gen = &mut self.gen;
            let pk = self
                .users
                .entry(id)
                .or_insert_with(|| Arbitrary::arbitrary(gen));
            self.keys.insert(Address::new(&pk.public()), pk.clone());
            pk.clone()
        }

        pub fn address(&mut self, id: usize) -> Address {
            Address::new(&self.user(id).public()).clone()
        }

        pub fn private(&self, public: &Address) -> PrivateKey {
            self.keys
                .get(public)
                .expect("private key should exist")
                .clone()
        }
    }

    /// Helper for building transactions in testing environment.
    struct TxBuilder {
        input: Vec<(Address, UtxoPointer)>,
        output: Vec<(usize, u64)>,
    }

    impl TxBuilder {
        // Create new builder.
        pub fn new() -> Self {
            TxBuilder {
                input: vec![],
                output: vec![],
            }
        }

        pub fn from_tx(&mut self, tx: &Transaction, idx: u32) -> &mut Self {
            use blockcfg::property::Transaction;
            let Output(address, _) = tx
                .outputs
                .get(idx as usize)
                .expect("expecting an output in the transaction");
            let utxo = UtxoPointer {
                transaction_id: tx.id(),
                output_index: idx,
            };
            self.input.push((*address, utxo));
            self
        }

        pub fn to(&mut self, uid: usize, value: u64) -> &mut Self {
            self.output.push((uid, value));
            self
        }

        pub fn build(&mut self, env: &mut Environment) -> SignedTransaction {
            use blockcfg::mock;
            use blockcfg::property::Transaction;
            let tx = mock::Transaction {
                inputs: self.input.iter().map(|(_, u)| u).cloned().collect(),
                outputs: self
                    .output
                    .iter()
                    .map(|(i, u)| Output(env.address(*i), Value(*u)))
                    .collect(),
            };
            let tx_id = tx.id();
            SignedTransaction {
                tx: tx,
                witnesses: self
                    .input
                    .iter()
                    .map(|(public, _)| Witness::new(tx_id, &env.private(public)))
                    .collect(),
            }
        }
    }

    #[test]
    fn can_pass_all_money_to_another() {
        use blockcfg::property::Ledger;
        let mut env = Environment::new();
        let (genesis, mut ledger) = env.genesis([(1, 100u64)].iter().cloned().collect());
        let stx = TxBuilder::new()
            .from_tx(&genesis, 0)
            .to(2, 100)
            .build(&mut env);
        let diff = match ledger.diff_transaction(&stx) {
            Ok(diff) => diff,
            Err(e) => panic!("Unexpected error {:#?}", e),
        };
        ledger.add(diff).unwrap();
    }

    #[test]
    fn can_split_money() {
        use blockcfg::property::Ledger;
        let mut env = Environment::new();
        let (genesis, mut ledger) = env.genesis([(1, 100u64)].iter().cloned().collect());
        let stx = TxBuilder::new()
            .from_tx(&genesis, 0)
            .to(1, 50)
            .to(1, 50)
            .build(&mut env);
        let diff = match ledger.diff_transaction(&stx) {
            Ok(diff) => diff,
            Err(e) => panic!("Unexpected error {:#?}", e),
        };
        ledger.add(diff).unwrap();
    }

    #[test]
    fn can_collect_money() {
        use blockcfg::property::Ledger;
        let mut env = Environment::new();
        let (genesis, mut ledger) = env.genesis([(1, 50u64), (2, 50u64)].iter().cloned().collect());
        let stx = TxBuilder::new()
            .from_tx(&genesis, 0)
            .from_tx(&genesis, 1)
            .to(1, 100)
            .build(&mut env);
        let diff = match ledger.diff_transaction(&stx) {
            Ok(diff) => diff,
            Err(e) => panic!("Unexpected error {:#?}", e),
        };
        ledger.add(diff).unwrap();
    }

    #[test]
    fn it_works() {
        use blockcfg::property::Ledger;
        let mut env = Environment::new();
        let (genesis, mut ledger) = env.genesis([(1, 100u64)].iter().cloned().collect());
        let stx = TxBuilder::new()
            .from_tx(&genesis, 0)
            .to(2, 50)
            .to(3, 50)
            .build(&mut env);
        let diff = match ledger.diff_transaction(&stx) {
            Ok(diff) => diff,
            Err(e) => panic!("Unexpected error {:#?}", e),
        };
        ledger.add(diff).unwrap();
        let stx2 = TxBuilder::new()
            .from_tx(&stx.tx, 0) // Get utxop in a better way
            .from_tx(&stx.tx, 1) // Get utxop in a better way
            .to(1, 100)
            .build(&mut env);
        let diff = match ledger.diff_transaction(&stx2) {
            Ok(diff) => diff,
            Err(e) => panic!("Unexpected error {:#?}", e),
        };
        ledger.add(diff).unwrap();
    }

    #[test]
    fn cant_loose_money() {
        use blockcfg::property::Ledger;
        let mut env = Environment::new();
        let (genesis, ledger) = env.genesis([(1, 100u64)].iter().cloned().collect());
        let stx = TxBuilder::new()
            .from_tx(&genesis, 0)
            .to(1, 50)
            .build(&mut env);
        match ledger.diff_transaction(&stx) {
            Ok(diff) => panic!("Unexpected transaction {:#?}", diff),
            Err(Error::TransactionSumIsNonZero(_, _)) => (),
            Err(e) => panic!("Unexpected error {:#?}", e),
        }
    }

    #[test]
    fn cant_generate_money() {
        use blockcfg::property::Ledger;
        let mut env = Environment::new();
        let (genesis, ledger) = env.genesis([(1, 100u64)].iter().cloned().collect());
        let stx = TxBuilder::new()
            .from_tx(&genesis, 0)
            .to(1, 150)
            .build(&mut env);
        match ledger.diff_transaction(&stx) {
            Ok(diff) => panic!("Unexpected transaction {:#?}", diff),
            Err(Error::TransactionSumIsNonZero(_, _)) => (),
            Err(e) => panic!("Unexpected error {:#?}", e),
        }
    }

    #[test]
    fn test_double_spend() {
        use blockcfg::property::Ledger;
        let mut env = Environment::new();
        let (genesis, ledger) = env.genesis([(1, 100u64)].iter().cloned().collect());
        let stx = TxBuilder::new()
            .from_tx(&genesis, 0)
            .from_tx(&genesis, 0)
            .to(2, 200)
            .build(&mut env);
        match ledger.diff_transaction(&stx) {
            Ok(diff) => panic!("Transaction succeeded {:#?}",diff),
            Err(Error::DoubleSpend(_,_)) => (),
            Err(e) => panic!("Unexpected error {:#?}", e),
        };
    }

    #[test]
    fn test_unresolved() {
        use blockcfg::property::Ledger;
        let mut env = Environment::new();
        let (genesis, mut ledger) = env.genesis([(1, 100u64)].iter().cloned().collect());
        let stx = TxBuilder::new()
            .from_tx(&genesis, 0)
            .to(2, 100)
            .build(&mut env);
        let diff = match ledger.diff_transaction(&stx) {
            Ok(diff) => diff,
            Err(e) => panic!("Unexpected error {:#?}", e),
        };
        ledger.add(diff).unwrap();
        match ledger.diff_transaction(&stx) {
            Ok(diff) => panic!("Unexpected success {:#?}", diff),
            Err(Error::InputDoesNotResolve(_)) => (),
            Err(e) => panic!("Unexpected error {:#?}", e),
        };
    }

}

#[cfg(test)]
mod quickcheck {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    quickcheck! {

        /// If witness was created for a transaction it verifies that.
        fn prop_witness_verfies_own_tx(pk: PrivateKey, tx: TransactionId) -> bool {
           let witness = Witness::new(tx, &pk);
           witness.verifies(tx)
        }

        fn witness_verifies_only_own_tx(pk: PrivateKey, tx1: Transaction, tx2: Transaction) -> bool {
            use blockcfg::property::Transaction;
            let witness1 = Witness::new(tx1.id(), &pk);
            let witness2 = Witness::new(tx2.id(), &pk);
            (witness1.verifies(tx2.id()) && witness1 == witness2) || (! witness1.verifies(tx2.id()))
        }

        /// id uniquelly identifies transaction.
        /// $$\forall tx1, tx2: id(tx1) == id(tx2) => tx1 == tx2$$
        fn prop_tx_id_uniqueness(tx1: Transaction, tx2: Transaction) -> bool {
            use blockcfg::property::Transaction;
            let id1 = tx1.id();
            let id2 = tx2.id();
            (id1 == id2 && tx1 == tx2) || (id1 != id2)
        }


    }

    impl Arbitrary for SlotId {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            SlotId(Arbitrary::arbitrary(g), Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for Hash {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut bytes = [0u8; 16];
            for byte in bytes.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            Hash(hash::Blake2b256::new(&bytes))
        }
    }

    impl Arbitrary for Value {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Value(Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for Address {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Address(Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for TransactionId {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            TransactionId(Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for Signature {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut signature = [0; crypto::SIGNATURE_SIZE];
            for byte in signature.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            Signature(crypto::Signature::from_bytes(signature))
        }
    }

    impl Arbitrary for PrivateKey {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut xprv = [0; crypto::XPRV_SIZE];
            for byte in xprv.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            PrivateKey(crypto::XPrv::normalize_bytes(xprv))
        }
    }

    impl Arbitrary for PublicKey {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut xpub = [0; crypto::XPUB_SIZE];
            for byte in xpub.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            PublicKey(crypto::XPub::from_bytes(xpub))
        }
    }

    impl Arbitrary for UtxoPointer {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            UtxoPointer {
                transaction_id: Arbitrary::arbitrary(g),
                output_index: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for Witness {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Witness {
                signature: Arbitrary::arbitrary(g),
                public_key: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for Output {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Output(Arbitrary::arbitrary(g), Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for Transaction {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Transaction {
                inputs: Arbitrary::arbitrary(g),
                outputs: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for SignedTransaction {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            SignedTransaction {
                tx: Arbitrary::arbitrary(g),
                witnesses: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for Block {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Block {
                slot_id: Arbitrary::arbitrary(g),
                parent_hash: Arbitrary::arbitrary(g),
                transactions: Arbitrary::arbitrary(g),
            }
        }
    }
}
