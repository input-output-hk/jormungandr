use crate::testing::FragmentBuilderError;
use chain_addr::Discrimination;
use chain_impl_mockchain::{
    account,
    fee::{FeeAlgorithm, LinearFee},
    transaction::{
        Balance, Input, InputOutputBuilder, Payload, PayloadSlice, TransactionSignDataHash,
        UnspecifiedAccountIdentifier, Witness,
    },
};
use jormungandr_lib::{
    crypto::{
        account::{Identifier, SigningKey},
        hash::Hash,
    },
    interfaces::{Address, Value},
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

    discrimination: Discrimination,
}

impl Wallet {
    pub fn generate<RNG>(rng: &mut RNG, discrimination: Discrimination) -> Self
    where
        RNG: CryptoRng + RngCore,
    {
        let signing_key = SigningKey::generate_extended(rng);
        let identifier = signing_key.identifier();
        Wallet {
            signing_key,
            identifier,
            internal_counter: account::SpendingCounter::zero(),
            discrimination,
        }
    }

    pub fn from_existing_account(bech32_str: &str, spending_counter: Option<u32>) -> Self {
        let signing_key = SigningKey::from_bech32_str(bech32_str).expect("bad bech32");
        let identifier = signing_key.identifier();
        Wallet {
            signing_key,
            identifier,
            internal_counter: spending_counter.unwrap_or(0).into(),
            discrimination: Discrimination::Test,
        }
    }

    pub fn save_to<W: std::io::Write>(&self, mut w: W) -> std::io::Result<()> {
        writeln!(w, "{}", self.signing_key().to_bech32_str())
    }

    pub fn address(&self) -> Address {
        self.identifier().to_address(self.discrimination).into()
    }

    pub fn increment_counter(&mut self) {
        let v: u32 = self.internal_counter.into();
        self.internal_counter = account::SpendingCounter::from(v + 1);
    }

    pub fn internal_counter(&self) -> account::SpendingCounter {
        self.internal_counter
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

    pub fn mk_witness(
        &self,
        block0_hash: &Hash,
        signing_data: &TransactionSignDataHash,
    ) -> Witness {
        Witness::new_account(&block0_hash.clone().into_hash(), signing_data, |d| {
            self.signing_key().as_ref().sign(d)
        })
    }

    pub fn add_input_with_value(&self, value: Value) -> Input {
        Input::from_account_single(
            self.identifier().to_inner(),
            self.internal_counter(),
            value.into(),
        )
    }

    pub fn add_input<'a, Extra: Payload>(
        &self,
        payload: PayloadSlice<'a, Extra>,
        iobuilder: &mut InputOutputBuilder,
        fees: &LinearFee,
    ) -> Result<(), FragmentBuilderError>
    where
        LinearFee: FeeAlgorithm,
    {
        let balance = iobuilder
            .get_balance_with_placeholders(payload, fees, 1, 0)
            .map_err(|_| FragmentBuilderError::CannotComputeBalance)?;
        let value = match balance {
            Balance::Negative(value) => value,
            Balance::Zero => return Err(FragmentBuilderError::TransactionAlreadyBalanced),
            Balance::Positive(value) => {
                return Err(FragmentBuilderError::TransactionAlreadyExtraValue(
                    value.into(),
                ))
            }
        };

        let input = Input::from_account_single(
            self.identifier().to_inner(),
            self.internal_counter(),
            value,
        );
        iobuilder.add_input(&input).unwrap();
        Ok(())
    }
}
