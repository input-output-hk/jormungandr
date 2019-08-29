use crate::wallet::{ErrorKind, Result, ResultExt};
use chain_addr::Discrimination;
use chain_impl_mockchain::{
    account,
    fee::{FeeAlgorithm, LinearFee},
    transaction::{
        AccountIdentifier, Balance, Input, Transaction, TransactionSignDataHash, Witness,
    },
    txbuilder::{self, TransactionBuilder},
    value::Value,
};
use jormungandr_lib::{
    crypto::{
        account::{Identifier, SigningKey},
        hash::Hash,
    },
    interfaces::Address,
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

    pub fn stake_key(&self) -> AccountIdentifier {
        AccountIdentifier::from_single_account(self.identifier().clone().to_inner())
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
    ) -> Result<Witness> {
        Ok(Witness::new_account(
            &block0_hash.clone().into_hash(),
            signing_data,
            self.internal_counter(),
            self.signing_key().as_ref(),
        ))
    }

    pub fn add_input<Extra: Clone>(
        &self,
        txbuilder: &mut TransactionBuilder<Extra>,
        fees: &LinearFee,
    ) -> Result<txbuilder::OutputPolicy>
    where
        LinearFee: FeeAlgorithm<Transaction<chain_addr::Address, Extra>>,
    {
        let identifier: chain_impl_mockchain::account::Identifier =
            self.identifier().to_inner().into();
        let balance = txbuilder
            .get_balance(fees)
            .chain_err(|| ErrorKind::CannotComputeBalance)?;
        let value = match balance {
            Balance::Negative(value) => value,
            Balance::Zero => bail!(ErrorKind::TransactionAlreadyBalanced),
            Balance::Positive(value) => {
                bail!(ErrorKind::TransactionAlreadyExtraValue(value.into()))
            }
        };

        // we are going to add an input
        let value = (value + Value(fees.coefficient))
            .chain_err(|| ErrorKind::CannotAddCostOfExtraInput(fees.coefficient))?;

        let input = Input::from_account_single(identifier, value);

        txbuilder.add_input(&input);

        Ok(txbuilder::OutputPolicy::Forget)
    }
}
