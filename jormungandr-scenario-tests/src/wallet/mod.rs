mod account;
mod utxo;

use crate::scenario::Wallet as WalletTemplate;
use chain_addr::Discrimination;
use chain_impl_mockchain::{
    certificate::{PoolId, SignedCertificate},
    fee::LinearFee,
    fragment::Fragment,
    transaction::UnspecifiedAccountIdentifier,
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{Address, Value},
};
use rand_core::{CryptoRng, RngCore};
use std::path::Path;

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
    pub fn save_to<P: AsRef<Path>>(&self, dir: P) -> std::io::Result<()> {
        let dir = dir.as_ref().join(self.template().alias());

        let file = std::fs::File::create(&dir)?;

        match &self.inner {
            Inner::Account(account) => account.save_to(file),
            Inner::UTxO(_utxo) => unimplemented!(),
        }
    }

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

    pub fn stake_key(&self) -> Option<UnspecifiedAccountIdentifier> {
        match &self.inner {
            Inner::Account(account) => Some(account.stake_key()),
            Inner::UTxO(_utxo) => unimplemented!(),
        }
    }

    pub fn delegation_cert_for_block0(&self, pool_id: PoolId) -> SignedCertificate {
        match &self.inner {
            Inner::Account(account) => account.delegation_cert_for_block0(pool_id),
            Inner::UTxO(_utxo) => unimplemented!(),
        }
    }

    pub(crate) fn template(&self) -> &WalletTemplate {
        &self.template
    }

    pub fn confirm_transaction(&mut self) {
        match &mut self.inner {
            Inner::Account(account) => account.increment_counter(),
            Inner::UTxO(_utxo) => unimplemented!(),
        }
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
        use chain_impl_mockchain::transaction::{InputOutputBuilder, NoExtra, Payload, TxBuilder};

        let mut iobuilder = InputOutputBuilder::empty();
        iobuilder.add_output(address.into(), value.into()).unwrap();

        let payload_data = NoExtra.payload_data();

        match &mut self.inner {
            Inner::Account(account) => account
                .add_input(payload_data.borrow(), &mut iobuilder, fees)
                .chain_err(|| "Cannot get inputs from the account")?,
            Inner::UTxO(_utxo) => unimplemented!(),
        };

        //let (_, tx) = txbuilder
        //    .seal_with_output_policy(fees, output_policy)
        //    .chain_err(|| "Cannot finalize the transaction")?;
        //let mut finalizer = TransactionFinalizer::new(tx.replace_extra(None));

        let ios = iobuilder.build();
        let txbuilder = TxBuilder::new()
            .set_nopayload()
            .set_ios(&ios.inputs, &ios.outputs);

        let sign_data = txbuilder.get_auth_data_for_witness().hash();

        let witness = match &mut self.inner {
            Inner::Account(account) => account
                .mk_witness(block0_hash, &sign_data)
                .chain_err(|| "Cannot create witness from account")?,
            Inner::UTxO(_utxo) => unimplemented!(),
        };

        let witnesses = vec![witness];
        let tx = txbuilder.set_witnesses(&witnesses).set_payload_auth(&());
        Ok(Fragment::Transaction(tx))
    }
}
