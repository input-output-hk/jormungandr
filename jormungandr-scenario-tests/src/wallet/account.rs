use super::{ErrorKind, Result, ResultExt};
use chain_addr::Discrimination;
use chain_impl_mockchain::{
    account,
    certificate::{PoolId, SignedCertificate, StakeDelegation},
    fee::{FeeAlgorithm, LinearFee},
    transaction::{
        AccountBindingSignature, AccountIdentifier, Balance, Input, InputOutputBuilder, Payload,
        PayloadSlice, TransactionSignDataHash, TxBuilder, Witness,
    },
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

    pub fn delegation_cert_for_block0(&self, pool_id: PoolId) -> SignedCertificate {
        let stake_delegation = StakeDelegation {
            account_id: self.stake_key(), // 2
            pool_id,                      // 1
        };
        let txb = TxBuilder::new()
            .set_payload(&stake_delegation)
            .set_ios(&[], &[])
            .set_witnesses(&[]);
        let auth_data = txb.get_auth_data();

        let sig = AccountBindingSignature::new(self.signing_key.as_ref(), &auth_data);
        SignedCertificate::StakeDelegation(stake_delegation, sig)
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

    pub fn add_input<'a, Extra: Payload>(
        &self,
        payload: PayloadSlice<'a, Extra>,
        iobuilder: &mut InputOutputBuilder,
        fees: &LinearFee,
    ) -> Result<()>
    where
        LinearFee: FeeAlgorithm,
    {
        let identifier: chain_impl_mockchain::account::Identifier =
            self.identifier().to_inner().into();
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

        let input = Input::from_account_single(identifier, value);

        iobuilder.add_input(&input).unwrap();

        Ok(())
    }
}
