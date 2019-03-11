use std::collections::HashMap;

use chain_addr::Address;
use chain_core::property;
use chain_core::property::testing;
use chain_crypto::{Ed25519Extended, SecretKey};
use quickcheck::{Arbitrary, Gen, StdGen};
use rand::prelude::*;

use crate::error::*;
use crate::ledger::*;
use crate::transaction::*;
use crate::update::TransactionsDiff;
use crate::value::*;

/// Helper structure that keeps the ledger together with key-pairs
/// that participate in communication. By having such type it's
/// possible to perform and generate new cryptographically signed
/// operations and verify them, without requiring users to mess with
/// keys on their own.
pub struct Environment {
    ledger: Ledger,
    gen: StdGen<rand::rngs::ThreadRng>,
    users: HashMap<usize, SecretKey<Ed25519Extended>>,
    keys: HashMap<Address, SecretKey<Ed25519Extended>>,
}

impl Environment {
    /// Create new environment.
    pub fn new() -> Self {
        let g = StdGen::new(rand::thread_rng(), 10);
        Environment {
            ledger: Ledger::new(HashMap::new()),
            gen: g,
            users: HashMap::new(),
            keys: HashMap::new(),
        }
    }

    pub fn random_new<G: Gen>(g: &mut G) -> Self {
        let mut env = Self::new();
        let ledger: Ledger = Arbitrary::arbitrary(g);
        use std::cmp::max;
        env.ledger = Ledger::new(
            ledger
                .unspent_outputs
                .iter()
                .enumerate()
                .map(|(n, (&i, Output(_, Value(value))))| {
                    (i, Output(env.address(n), Value(max(1, *value))))
                })
                .collect(),
        );
        env
    }

    /// Get users private key based on the user's index,
    /// if there is no such a user yet - the user will be
    /// generated.
    pub fn user(&mut self, id: usize) -> SecretKey<Ed25519Extended> {
        use chain_addr::{Discrimination, Kind};
        let gen = &mut self.gen;
        let pk = self
            .users
            .entry(id)
            .or_insert_with(|| crate::key::test::arbitrary_secret_key(gen));
        let address = Address(Discrimination::Production, Kind::Single(pk.to_public()));
        self.keys.insert(address, pk.clone());
        pk.clone()
    }

    /// Get user's address based it's index. If user does
    /// not exist, it will be generated.
    pub fn address(&mut self, id: usize) -> Address {
        use chain_addr::{Discrimination, Kind};
        let address = Address(
            Discrimination::Production,
            Kind::Single(self.user(id).to_public()),
        );
        address
    }

    /// Get user's private key based on user's address.
    /// panics if user is not in the environment yet.
    pub fn private(&mut self, public: &Address) -> SecretKey<Ed25519Extended> {
        self.keys
            .get(public)
            .expect("Public key should be registered in env first.")
            .clone()
    }
}

impl property::Ledger for Environment {
    type Update = TransactionsDiff;
    type Error = Error;
    type Transaction = SignedTransaction<Address>;

    fn input<'a>(
        &'a self,
        input: &<self::SignedTransaction<Address> as property::Transaction>::Input,
    ) -> Result<&'a <self::SignedTransaction<Address> as property::Transaction>::Output, Self::Error>
    {
        self.ledger.input(input)
    }

    fn diff_transaction(
        &self,
        transaction: &SignedTransaction<Address>,
    ) -> Result<Self::Update, Self::Error> {
        self.ledger.diff_transaction(transaction)
    }

    fn diff<'a, I>(&self, transactions: I) -> Result<Self::Update, Self::Error>
    where
        I: IntoIterator<Item = &'a SignedTransaction<Address>> + Sized,
    {
        self.ledger.diff(transactions)
    }

    fn apply(&mut self, diff: Self::Update) -> Result<&mut Self, Self::Error> {
        match self.ledger.apply(diff) {
            Ok(_) => Ok(self),
            Err(e) => Err(e),
        }
    }
}

impl testing::GenerateTransaction<SignedTransaction<Address>> for Environment {
    fn generate_transaction<G>(&mut self, g: &mut G) -> SignedTransaction<Address>
    where
        G: Gen,
    {
        use crate::transaction as mock;
        use chain_core::property::Transaction;
        use std::cmp::{max, min};
        // select some unspent inputs for transaction.
        let inputs_outputs: Vec<(mock::UtxoPointer, mock::Output<Address>)> = self
            .ledger
            .unspent_outputs
            .iter()
            .filter(|_| Arbitrary::arbitrary(g))
            .map(|(i, o)| (i.clone(), o.clone()))
            .collect();
        // find out how much money should we split.
        let mut output_sum: u64 = inputs_outputs
            .iter()
            .map(|(_, Output(_, Value(v)))| v)
            .sum();
        // generate output vector.
        let mut outputs = Vec::new();
        loop {
            if output_sum == 0 {
                break;
            }
            let address = self.address(Arbitrary::arbitrary(g));
            let value = min(max(1, Arbitrary::arbitrary(g)), output_sum);
            outputs.push(Output(address, Value(value)));
            output_sum = output_sum - value;
        }
        let tx = mock::Transaction {
            inputs: inputs_outputs
                .iter()
                .cloned()
                .map(|(i, _)| i.clone())
                .collect(),
            outputs: outputs,
        };
        let tx_id = tx.id();
        let mut witnesses = Vec::with_capacity(tx.inputs.len());
        for (_, Output(public, _)) in inputs_outputs.iter() {
            witnesses.push(Witness::new(&tx_id, &self.private(public)));
        }
        SignedTransaction {
            transaction: tx,
            witnesses: witnesses,
        }
    }
}

#[derive(Clone, Debug)]
struct LedgerWithValidTransaction(
    pub testing::LedgerWithValidTransaction<Ledger, SignedTransaction<Address>>,
);

impl Arbitrary for LedgerWithValidTransaction {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut env = Environment::random_new(g);
        use chain_core::property::testing::GenerateTransaction;
        let signed_tx = env.generate_transaction(g);
        LedgerWithValidTransaction(testing::LedgerWithValidTransaction(env.ledger, signed_tx))
    }
}

#[cfg(test)]
quickcheck! {
    fn prop_valid_tx_succeeds(l: LedgerWithValidTransaction) -> bool {
       let LedgerWithValidTransaction(inner) = l;
       let mut v = inner.clone();
       testing::prop_good_transactions_succeed(&mut v)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_valid_tx() -> () {
        let mut g = StdGen::new(thread_rng(), 10);
        let mut env = Environment::random_new(&mut g);
        testing::run_valid_transactions(&mut g, &mut env, 100);
    }
}
