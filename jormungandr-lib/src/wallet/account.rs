use super::WalletError;
use crate::{
    crypto::{
        account::{Identifier, SigningKey},
        hash::Hash,
    },
    interfaces::{Address, Value},
};
use chain_addr::Discrimination;
use chain_impl_mockchain::{
    account,
    certificate::{PoolId, SignedCertificate, StakeDelegation},
    fee::{FeeAlgorithm, LinearFee},
    fragment::Fragment,
    transaction::{
        AccountBindingSignature, Balance, Input, InputOutputBuilder, NoExtra, Payload,
        PayloadSlice, TransactionSignDataHash, TxBuilder, UnspecifiedAccountIdentifier, Witness,
    },
};

use rand_core::{CryptoRng, RngCore};

/// wallet for an account
#[derive(Debug, Clone)]
pub struct Wallet {
    /// the spending key
    signing_key: SigningKey,

    /// the identifier of the account
    identifier: Identifier,

    /// the counter as we know of this value needs to be in sync
    /// with what is in the blockchain
    internal_counter: account::SpendingCounter,
}

impl Wallet {
    pub fn generate<RNG>(rng: &mut RNG) -> Self
    where
        RNG: CryptoRng + RngCore,
    {
        let signing_key = SigningKey::generate(rng);
        let identifier = signing_key.identifier();
        Wallet {
            signing_key,
            identifier,
            internal_counter: account::SpendingCounter::zero(),
        }
    }

    pub fn from_existing_account(bech32_str: &str, spending_counter: Option<u32>) -> Self {
        let signing_key = SigningKey::from_bech32_str(bech32_str).expect("bad bech32");
        let identifier = signing_key.identifier();
        Wallet {
            signing_key,
            identifier,
            internal_counter: spending_counter.unwrap_or_else(|| 0).into(),
        }
    }

    pub fn save_to<W: std::io::Write>(&self, mut w: W) -> std::io::Result<()> {
        writeln!(w, "{}", self.signing_key().to_bech32_str())
    }

    pub fn address(&self, discrimination: Discrimination) -> Address {
        self.identifier().to_address(discrimination).into()
    }

    pub fn increment_counter(&mut self) {
        let v: u32 = self.internal_counter.into();
        self.internal_counter = account::SpendingCounter::from(v + 1);
    }

    pub fn internal_counter(&self) -> &account::SpendingCounter {
        &self.internal_counter
    }

    pub fn stake_key(&self) -> UnspecifiedAccountIdentifier {
        UnspecifiedAccountIdentifier::from_single_account(self.identifier().clone().to_inner())
    }

    pub fn identifier(&self) -> &Identifier {
        &self.identifier
    }

    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    pub fn delegation_cert_for_block0(&self, pool_id: PoolId) -> SignedCertificate {
        let stake_delegation = StakeDelegation {
            account_id: self.stake_key(), // 2
            delegation: chain_impl_mockchain::account::DelegationType::Full(pool_id), // 1
        };
        let txb = TxBuilder::new()
            .set_payload(&stake_delegation)
            .set_ios(&[], &[])
            .set_witnesses(&[]);
        let auth_data = txb.get_auth_data();

        let sig = AccountBindingSignature::new_single(&auth_data, |d| {
            self.signing_key.as_ref().sign_slice(&d.0)
        });
        SignedCertificate::StakeDelegation(stake_delegation, sig)
    }

    pub fn mk_witness(
        &self,
        block0_hash: &Hash,
        signing_data: &TransactionSignDataHash,
    ) -> Witness {
        Witness::new_account(
            &block0_hash.clone().into_hash(),
            signing_data,
            self.internal_counter(),
            |d| self.signing_key().as_ref().sign(d),
        )
    }

    pub fn transaction_to(
        &self,
        block0_hash: &Hash,
        fees: &LinearFee,
        address: Address,
        value: Value,
    ) -> Result<Fragment, WalletError> {
        let mut iobuilder = InputOutputBuilder::empty();
        iobuilder.add_output(address.into(), value.into()).unwrap();

        let payload_data = NoExtra.payload_data();
        self.add_input(payload_data.borrow(), &mut iobuilder, fees)?;

        let ios = iobuilder.build();
        let txbuilder = TxBuilder::new()
            .set_nopayload()
            .set_ios(&ios.inputs, &ios.outputs);

        let sign_data = txbuilder.get_auth_data_for_witness().hash();
        let witness = self.mk_witness(block0_hash, &sign_data);
        let witnesses = vec![witness];
        let tx = txbuilder.set_witnesses(&witnesses).set_payload_auth(&());
        Ok(Fragment::Transaction(tx))
    }

    pub fn add_input<'a, Extra: Payload>(
        &self,
        payload: PayloadSlice<'a, Extra>,
        iobuilder: &mut InputOutputBuilder,
        fees: &LinearFee,
    ) -> Result<(), WalletError>
    where
        LinearFee: FeeAlgorithm,
    {
        let balance = iobuilder
            .get_balance_with_placeholders(payload, fees, 1, 0)
            .map_err(|_| WalletError::CannotComputeBalance)?;
        let value = match balance {
            Balance::Negative(value) => value,
            Balance::Zero => return Err(WalletError::TransactionAlreadyBalanced),
            Balance::Positive(value) => {
                return Err(WalletError::TransactionAlreadyExtraValue(value.into()))
            }
        };

        let input = Input::from_account_single(self.identifier().to_inner().into(), value);
        iobuilder.add_input(&input).unwrap();
        Ok(())
    }
}
