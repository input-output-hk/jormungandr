use crate::data::{Choice, Value};
use bip39::{dictionary, Entropy, Type};
use chain_addr::{AddressReadable, Discrimination};
use chain_core::property::Deserialize;
use chain_core::property::Fragment as _;
use chain_impl_mockchain::{
    fragment::{Fragment, FragmentId},
    transaction::Input,
};
use hdkeygen::account::AccountId;
use jormungandr_lib::interfaces::AccountIdentifier;
use std::collections::HashMap;
use std::str::FromStr;
use wallet::Settings;
use wallet_core::Conversion;
use wallet_core::Wallet as Inner;
pub use wallet_core::{Error as WalletError, Proposal};

pub struct Wallet {
    proposals: Vec<Proposal>,
    inner: Inner,
}

impl Wallet {
    pub fn generate(words_length: Type) -> Result<Self, WalletError> {
        let entropy = Entropy::generate(words_length, rand::random);
        let mnemonics = entropy.to_mnemonics().to_string(&dictionary::ENGLISH);
        Self::recover(&mnemonics, "iapyx".as_bytes())
    }

    pub fn recover(mnemonics: &str, password: &[u8]) -> Result<Self, WalletError> {
        Ok(Self {
            inner: Inner::recover(mnemonics, password)?,
            proposals: vec![],
        })
    }

    pub fn account(&self, discrimination: chain_addr::Discrimination) -> chain_addr::Address {
        self.inner.account(discrimination)
    }

    pub fn id(&self) -> AccountId {
        self.inner.id()
    }

    pub fn retrieve_funds(&mut self, block0_bytes: &[u8]) -> Result<wallet::Settings, WalletError> {
        self.inner.retrieve_funds(block0_bytes)
    }

    pub fn convert(&mut self, settings: Settings) -> Conversion {
        self.inner.convert(settings)
    }

    pub fn conversion_fragment_ids(&mut self, settings: Settings) -> Vec<FragmentId> {
        let conversion = self.convert(settings);
        conversion
            .transactions()
            .iter()
            .map(|x| {
                let fragment = Fragment::deserialize(x.as_slice()).unwrap();
                self.remove_pending_transaction(&fragment.id());
                fragment.id()
            })
            .collect()
    }

    pub fn confirm_all_transactions(&mut self) {
        for (id, _) in self.pending_transactions().clone() {
            self.confirm_transaction(id)
        }
    }

    pub fn confirm_transaction(&mut self, id: FragmentId) {
        self.inner.confirm_transaction(id);
    }

    pub fn pending_transactions(&self) -> &HashMap<FragmentId, Vec<Input>> {
        &self.inner.pending_transactions()
    }

    pub fn remove_pending_transaction(&mut self, id: &FragmentId) -> Option<Vec<Input>> {
        self.inner.remove_pending_transaction(id)
    }

    pub fn total_value(&self) -> Value {
        self.inner.total_value()
    }

    pub fn set_state(&mut self, value: Value, counter: u32) {
        self.inner.set_state(value, counter);
    }

    pub fn vote(
        &mut self,
        settings: Settings,
        proposal: &Proposal,
        choice: Choice,
    ) -> Result<Box<[u8]>, WalletError> {
        self.inner.vote(settings, proposal, choice)
    }

    pub fn set_proposals(&mut self, proposals: Vec<Proposal>) {
        self.proposals = proposals;
    }

    pub fn identifier(&self) -> AccountIdentifier {
        AccountIdentifier::from_str(
            &AddressReadable::from_address("ca", &self.account(Discrimination::Test)).to_string(),
        )
        .unwrap()
    }
}

impl std::fmt::Debug for Wallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.identifier().to_string())
    }
}
