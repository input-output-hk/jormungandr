use crate::fragment::FragmentBuilderError;
use chain_addr::Discrimination;
use chain_impl_mockchain::{
    account::SpendingCounter,
    accounting::account::SpendingCounterIncreasing,
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

    /// the counter as we know of this value needs to be in sync
    /// with what is in the blockchain
    internal_counters: SpendingCounterIncreasing,

    discrimination: Discrimination,
}

impl Wallet {
    pub fn generate<RNG>(rng: &mut RNG, discrimination: Discrimination) -> Self
    where
        RNG: CryptoRng + RngCore,
    {
        let signing_key = SigningKey::generate_extended(rng);
        Wallet {
            signing_key,
            internal_counters: SpendingCounterIncreasing::default(),
            discrimination,
        }
    }

    pub fn from_secret_key(
        signing_key: SigningKey,
        internal_counters: SpendingCounterIncreasing,
        discrimination: Discrimination,
    ) -> Self {
        Wallet {
            signing_key,
            internal_counters,
            discrimination,
        }
    }

    pub fn from_existing_account(
        bech32_str: &str,
        spending_counter: Option<SpendingCounter>,
        discrimination: Discrimination,
    ) -> Self {
        let signing_key = SigningKey::from_bech32_str(bech32_str).expect("bad bech32");
        Wallet {
            signing_key,
            internal_counters: SpendingCounterIncreasing::new_from_counter(
                spending_counter
                    .map(Into::into)
                    .unwrap_or_else(SpendingCounter::zero),
            ),
            discrimination,
        }
    }

    pub fn save_to<W: std::io::Write>(&self, mut w: W) -> std::io::Result<()> {
        writeln!(w, "{}", self.signing_key().to_bech32_str())
    }

    pub fn address(&self) -> Address {
        self.identifier().to_address(self.discrimination).into()
    }

    pub fn set_counter(&mut self, counter: SpendingCounter) {
        let mut counters = self.internal_counters.get_valid_counters();
        counters[counter.lane()] = counter;
        self.internal_counters = SpendingCounterIncreasing::new_from_counters(counters).unwrap();
    }

    pub fn increment_counter(&mut self, lane: usize) {
        self.internal_counters
            .next_verify(self.internal_counters.get_valid_counters()[lane])
            .unwrap();
    }

    pub fn decrement_counter(&mut self, lane: usize) {
        self.set_counter(SpendingCounter::from(
            <u32>::from(self.internal_counters()[lane]) - 1,
        ))
    }

    pub fn spending_counter(&self) -> &SpendingCounterIncreasing {
        &self.internal_counters
    }

    /// Use the default counter
    pub fn internal_counter(&self) -> SpendingCounter {
        self.internal_counters.get_valid_counter()
    }

    pub fn internal_counters(&self) -> Vec<SpendingCounter> {
        self.internal_counters.get_valid_counters()
    }

    pub fn stake_key(&self) -> UnspecifiedAccountIdentifier {
        UnspecifiedAccountIdentifier::from_single_account(self.identifier().to_inner())
    }

    pub fn identifier(&self) -> Identifier {
        self.signing_key.identifier()
    }

    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    pub fn mk_witness(
        &self,
        block0_hash: &Hash,
        signing_data: &TransactionSignDataHash,
    ) -> Witness {
        Witness::new_account(
            &(*block0_hash).into_hash(),
            signing_data,
            self.internal_counters.get_valid_counter(),
            |d| self.signing_key().as_ref().sign(d),
        )
    }

    pub fn add_input_with_value(&self, value: Value) -> Input {
        Input::from_account_single(self.identifier().to_inner(), value.into())
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

        let input = Input::from_account_single(self.identifier().to_inner(), value);
        iobuilder.add_input(&input).unwrap();
        Ok(())
    }
}
