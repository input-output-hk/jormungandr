mod account;
mod utxo;

use crate::scenario::Wallet as WalletTemplate;
use chain_addr::Discrimination;
use chain_impl_mockchain::{fee::LinearFee, fragment::Fragment, transaction::AccountIdentifier};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{Address, Value},
};
use rand_core::{CryptoRng, RngCore};

error_chain! {
    foreign_links {
        Io(::std::io::Error);
    }

    errors {
        CannotAddInput {
            description("Cannot add input to the transaction"),
        }

        CannotMakeWitness {
            description("Cannot make witness for the transaction"),
        }

        CannotComputeBalance {
            description("Cannot compute the transaction's balance")
        }

        CannotAddCostOfExtraInput(fee: u64) {
            description("cannot compute the new fees for adding the new input")
            display("Cannot compute the new fees of {} for a new input", fee),
        }

        TransactionAlreadyBalanced {
            description("Transaction already balanced")
        }

        TransactionAlreadyExtraValue(value: Value) {
            description("Transaction has already more inputs than needed"),
            display("The transaction has {} value extra than necessary", value),
        }
    }
}

#[derive(Debug, Clone)]
enum Inner {
    Account(account::Wallet),
    UTxO(utxo::Wallet),
}

/// wallet to utilise when testing jormungandr
///
/// This can be used for a faucet
#[derive(Debug, Clone)]
pub struct Wallet {
    inner: Inner,

    template: WalletTemplate,
}

impl Wallet {
    pub fn generate_account<RNG>(template: WalletTemplate, rng: &mut RNG) -> Self
    where
        RNG: CryptoRng + RngCore,
    {
        Wallet {
            inner: Inner::Account(account::Wallet::generate(rng)),
            template,
        }
    }

    pub fn generate_utxo<RNG>(template: WalletTemplate, rng: &mut RNG) -> Self
    where
        RNG: CryptoRng + RngCore,
    {
        Wallet {
            inner: Inner::UTxO(utxo::Wallet::generate(rng)),
            template,
        }
    }

    pub fn address(&self, discrimination: Discrimination) -> Address {
        match &self.inner {
            Inner::Account(account) => account.address(discrimination),
            Inner::UTxO(_utxo) => unimplemented!(),
        }
    }

    pub fn stake_key(&self) -> Option<AccountIdentifier> {
        match &self.inner {
            Inner::Account(account) => Some(account.stake_key()),
            Inner::UTxO(_utxo) => unimplemented!(),
        }
    }

    pub(crate) fn template(&self) -> &WalletTemplate {
        &self.template
    }

    /// simple function to create a transaction with only one output to the given
    /// address and of the given Value.
    ///
    pub fn transaction_to(
        &mut self,
        block0_hash: &Hash,
        fees: &LinearFee,
        address: Address,
        value: Value,
    ) -> Result<Fragment> {
        use chain_impl_mockchain::txbuilder::{
            GeneratedTransaction, TransactionBuilder, TransactionFinalizer,
        };

        let mut txbuilder = TransactionBuilder::new();

        txbuilder.add_output(address.into(), value.into());

        let output_policy = match &mut self.inner {
            Inner::Account(account) => account
                .add_input(&mut txbuilder, fees)
                .chain_err(|| "Cannot get inputs from the account")?,
            Inner::UTxO(_utxo) => unimplemented!(),
        };

        let (_, tx) = txbuilder
            .finalize(fees, output_policy)
            .chain_err(|| "Cannot finalize the transaction")?;
        let mut finalizer = TransactionFinalizer::new_trans(tx);

        let sign_data = finalizer.get_txid();

        let witness = match &mut self.inner {
            Inner::Account(account) => account
                .mk_witness(block0_hash, &sign_data)
                .chain_err(|| "Cannot create witness from account")?,
            Inner::UTxO(_utxo) => unimplemented!(),
        };

        finalizer
            .set_witness(0, witness)
            .chain_err(|| "Cannot add witness")?;

        match finalizer
            .build()
            .chain_err(|| "Cannot generate the finalized transaction")?
        {
            GeneratedTransaction::Type1(transaction) => Ok(Fragment::Transaction(transaction)),
            _ => unimplemented!(),
        }
    }
}
