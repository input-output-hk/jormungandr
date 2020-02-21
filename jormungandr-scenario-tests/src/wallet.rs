use crate::scenario::Wallet as WalletTemplate;
use chain_addr::Discrimination;
use chain_impl_mockchain::{
    certificate::{PoolId, SignedCertificate, StakeDelegation},
    fee::{FeeAlgorithm, LinearFee},
    fragment::Fragment,
    transaction::{
        AccountBindingSignature, Balance, Input, InputOutputBuilder, NoExtra, Payload,
        PayloadSlice, TxBuilder, UnspecifiedAccountIdentifier,
    },
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{Address, Value},
    wallet::{account::Wallet as AccountWallet, utxo::Wallet as UtxOWallet, Wallet as Inner},
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
            _ => unimplemented!(),
        }
    }

    pub fn generate_account<RNG>(template: WalletTemplate, rng: &mut RNG) -> Self
    where
        RNG: CryptoRng + RngCore,
    {
        Wallet {
            inner: Inner::Account(AccountWallet::generate(rng)),
            template,
        }
    }

    pub fn generate_utxo<RNG>(template: WalletTemplate, rng: &mut RNG) -> Self
    where
        RNG: CryptoRng + RngCore,
    {
        Wallet {
            inner: Inner::UTxO(UtxOWallet::generate(rng)),
            template,
        }
    }

    pub fn address(&self, discrimination: Discrimination) -> Address {
        match &self.inner {
            Inner::Account(account) => account.address(discrimination),
            _ => unimplemented!(),
        }
    }

    pub fn stake_key(&self) -> Option<UnspecifiedAccountIdentifier> {
        match &self.inner {
            Inner::Account(account) => Some(account.stake_key()),
            _ => unimplemented!(),
        }
    }

    pub fn delegation_cert_for_block0(&self, pool_id: PoolId) -> SignedCertificate {
        match &self.inner {
            Inner::Account(_) => self.delegation_cert_account_for_block0(pool_id),
            _ => unimplemented!(),
        }
    }

    pub fn delegation_cert_account_for_block0(&self, pool_id: PoolId) -> SignedCertificate {
        let stake_delegation = StakeDelegation {
            account_id: self.stake_key().unwrap(), // 2
            delegation: chain_impl_mockchain::account::DelegationType::Full(pool_id), // 1
        };
        let txb = TxBuilder::new()
            .set_payload(&stake_delegation)
            .set_ios(&[], &[])
            .set_witnesses(&[]);
        let auth_data = txb.get_auth_data();

        match &self.inner {
            Inner::Account(account) => {
                let sig = AccountBindingSignature::new_single(&auth_data, |d| {
                    account.signing_key().as_ref().sign_slice(&d.0)
                });
                SignedCertificate::StakeDelegation(stake_delegation, sig)
            }
            _ => unimplemented!(),
        }
    }

    pub(crate) fn template(&self) -> &WalletTemplate {
        &self.template
    }

    pub fn confirm_transaction(&mut self) {
        self.inner.confirm_transaction()
    }

    pub fn identifier(&mut self) -> chain_impl_mockchain::account::Identifier {
        match &mut self.inner {
            Inner::Account(account) => account.identifier().to_inner().into(),
            _ => unimplemented!(),
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
        let mut iobuilder = InputOutputBuilder::empty();
        iobuilder.add_output(address.into(), value.into()).unwrap();

        let payload_data = NoExtra.payload_data();
        self.add_input(payload_data.borrow(), &mut iobuilder, fees);

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
            Inner::Account(account) => account.mk_witness(block0_hash, &sign_data),
            _ => unimplemented!(),
        };

        let witnesses = vec![witness];
        let tx = txbuilder.set_witnesses(&witnesses).set_payload_auth(&());
        Ok(Fragment::Transaction(tx))
    }

    pub fn add_input<'a, Extra: Payload>(
        &mut self,
        payload: PayloadSlice<'a, Extra>,
        iobuilder: &mut InputOutputBuilder,
        fees: &LinearFee,
    ) -> Result<()>
    where
        LinearFee: FeeAlgorithm,
    {
        let balance = iobuilder
            .get_balance_with_placeholders(payload, fees, 1, 0)
            .chain_err(|| ErrorKind::CannotComputeBalance)?;
        let value = match balance {
            Balance::Negative(value) => value,
            Balance::Zero => bail!(ErrorKind::TransactionAlreadyBalanced),
            Balance::Positive(value) => {
                bail!(ErrorKind::TransactionAlreadyExtraValue(value.into()))
            }
        };

        let input = Input::from_account_single(self.identifier(), value);

        iobuilder.add_input(&input).unwrap();

        Ok(())
    }
}
