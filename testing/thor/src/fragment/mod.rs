pub use self::{
    chain_sender::FragmentChainSender,
    export::{FragmentExporter, FragmentExporterError},
    initial_certificates::{signed_delegation_cert, signed_stake_pool_cert, vote_plan_cert},
    persistent_log::{write_into_persistent_log, PersistentLogViewer},
    sender::{BlockDateGenerator, FragmentSender, FragmentSenderError},
    setup::{DummySyncNode, FragmentSenderSetup, FragmentSenderSetupBuilder, VerifyStrategy},
    verifier::{ExitStrategy as VerifyExitStrategy, FragmentVerifier, FragmentVerifierError},
};
use crate::{
    stake_pool::StakePool,
    wallet::{account::Wallet as AccountWallet, Wallet},
};
use chain_crypto::{Ed25519, SecretKey};
#[cfg(feature = "evm")]
use chain_impl_mockchain::certificate::EvmMapping;
#[cfg(feature = "evm")]
use chain_impl_mockchain::evm::EvmTransaction;
use chain_impl_mockchain::{
    block::BlockDate,
    certificate::{
        PoolId, UpdateProposal, UpdateVote, VoteCast, VotePlan, VoteTally, VoteTallyPayload,
    },
    fee::{FeeAlgorithm, LinearFee},
    fragment::Fragment,
    testing::{
        data::{StakePool as StakePoolLib, Wallet as WalletLib},
        scenario::FragmentFactory,
        WitnessMode,
    },
    transaction::{InputOutputBuilder, TxBuilder},
    vote::{Choice, Payload, PayloadType},
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{Address, Initial, Value},
};
use rand_chacha::ChaCha20Rng;
use rand_core::SeedableRng;
use thiserror::Error;
pub use transaction_utils::TransactionHash;

mod chain_sender;
mod export;
mod initial_certificates;
mod persistent_log;
mod sender;
mod setup;
mod transaction_utils;
mod verifier;

#[derive(Error, Debug)]
pub enum FragmentBuilderError {
    #[error("cannot compute the transaction's balance")]
    CannotComputeBalance,
    #[error("Cannot compute the new fees of {0} for a new input")]
    CannotAddCostOfExtraInput(u64),
    #[error("transaction already balanced")]
    TransactionAlreadyBalanced,
    #[error("the transaction has {0} value extra than necessary")]
    TransactionAlreadyExtraValue(Value),
}

pub struct FragmentBuilder {
    fragment_factory: FragmentFactory,
    valid_until: BlockDate,
}

impl FragmentBuilder {
    pub fn new(block0_hash: &Hash, fees: &LinearFee, valid_until: BlockDate) -> Self {
        Self {
            fragment_factory: FragmentFactory::new(block0_hash.into_hash(), fees.clone()),
            valid_until,
        }
    }

    pub fn witness_mode(mut self, witness_mode: WitnessMode) -> Self {
        self.fragment_factory = self.fragment_factory.witness_mode(witness_mode);
        self
    }

    pub fn transaction(
        &self,
        from: &Wallet,
        address: Address,
        value: Value,
    ) -> Result<Fragment, FragmentBuilderError> {
        self.transaction_to_many(from, &[address], value)
    }

    pub fn transaction_to_many(
        &self,
        from: &Wallet,
        addresses: &[Address],
        value: Value,
    ) -> Result<Fragment, FragmentBuilderError> {
        let mut iobuilder = InputOutputBuilder::empty();

        for address in addresses {
            iobuilder
                .add_output(address.clone().into(), value.into())
                .unwrap();
        }

        let value_u64: u64 = value.into();
        let input_without_fees: Value = (value_u64 * addresses.len() as u64).into();
        let input_value = self
            .fragment_factory
            .fee
            .calculate(None, 1, addresses.len() as u8)
            + input_without_fees.into();
        let input = from.add_input_with_value(input_value.unwrap().into());
        iobuilder.add_input(&input).unwrap();

        let ios = iobuilder.build();
        let txbuilder = TxBuilder::new()
            .set_nopayload()
            .set_expiry_date(self.valid_until)
            .set_ios(&ios.inputs, &ios.outputs);

        let sign_data = txbuilder.get_auth_data_for_witness().hash();
        let witness = from.mk_witness(&self.fragment_factory.block0_hash.into(), &sign_data);
        let witnesses = vec![witness];
        let tx = txbuilder.set_witnesses(&witnesses).set_payload_auth(&());
        Ok(Fragment::Transaction(tx))
    }

    pub fn full_delegation_cert_for_block0(
        valid_until: BlockDate,
        wallet: &Wallet,
        pool_id: PoolId,
    ) -> Initial {
        Initial::Cert(signed_delegation_cert(wallet, valid_until, pool_id).into())
    }

    pub fn stake_pool_registration(&self, funder: &Wallet, stake_pool: &StakePool) -> Fragment {
        let inner_wallet = funder.clone().into();
        self.fragment_factory.stake_pool_registration(
            self.valid_until,
            &inner_wallet,
            &stake_pool.clone().into(),
        )
    }

    pub fn delegation(&self, from: &Wallet, stake_pool: &StakePool) -> Fragment {
        let inner_wallet = from.clone().into();
        self.fragment_factory.delegation(
            self.valid_until,
            &inner_wallet,
            &stake_pool.clone().into(),
        )
    }

    pub fn delegation_remove(&self, from: &Wallet) -> Fragment {
        let inner_wallet = from.clone().into();
        self.fragment_factory
            .delegation_remove(self.valid_until, &inner_wallet)
    }

    pub fn delegation_to_many(
        &self,
        from: &Wallet,
        distribution: Vec<(&StakePool, u8)>,
    ) -> Fragment {
        let inner_wallet = from.clone().into();
        let inner_stake_pools: Vec<StakePoolLib> = distribution
            .iter()
            .cloned()
            .map(|(x, _)| {
                let inner_stake_pool: StakePoolLib = x.clone().into();
                inner_stake_pool
            })
            .collect();

        let mut inner_distribution: Vec<(&StakePoolLib, u8)> = Vec::new();

        for (inner_stake_pool, (_, factor)) in inner_stake_pools.iter().zip(distribution) {
            inner_distribution.push((inner_stake_pool, factor));
        }

        self.fragment_factory.delegation_to_many(
            self.valid_until,
            &inner_wallet,
            &inner_distribution[..],
        )
    }

    pub fn owner_delegation(&self, from: &Wallet, stake_pool: &StakePool) -> Fragment {
        let inner_wallet = from.clone().into();
        self.fragment_factory.owner_delegation(
            self.valid_until,
            &inner_wallet,
            &stake_pool.clone().into(),
        )
    }

    pub fn stake_pool_retire(&self, owners: Vec<&Wallet>, stake_pool: &StakePool) -> Fragment {
        let inner_owners: Vec<WalletLib> = owners
            .iter()
            .cloned()
            .map(|x| {
                let wallet: WalletLib = x.clone().into();
                wallet
            })
            .collect();

        let ref_inner_owners: Vec<&WalletLib> = inner_owners.iter().collect();
        self.fragment_factory.stake_pool_retire(
            self.valid_until,
            ref_inner_owners,
            &stake_pool.clone().into(),
        )
    }

    pub fn stake_pool_update(
        &self,
        owners: Vec<&Wallet>,
        old_stake_pool: &StakePool,
        new_stake_pool: &StakePool,
    ) -> Fragment {
        let inner_owners: Vec<WalletLib> = owners
            .iter()
            .cloned()
            .map(|x| {
                let wallet: WalletLib = x.clone().into();
                wallet
            })
            .collect();

        let ref_inner_owners: Vec<&WalletLib> = inner_owners.iter().collect();
        self.fragment_factory.stake_pool_update(
            self.valid_until,
            ref_inner_owners,
            &old_stake_pool.clone().into(),
            new_stake_pool.clone().into(),
        )
    }

    pub fn vote_plan(&self, wallet: &Wallet, vote_plan: &VotePlan) -> Fragment {
        let inner_wallet = wallet.clone().into();
        self.fragment_factory
            .vote_plan(self.valid_until, &inner_wallet, vote_plan.clone())
    }

    pub fn vote_cast(
        &self,
        wallet: &Wallet,
        vote_plan: &VotePlan,
        proposal_index: u8,
        choice: &Choice,
    ) -> Fragment {
        match vote_plan.payload_type() {
            PayloadType::Public => self.public_vote_cast(wallet, vote_plan, proposal_index, choice),
            PayloadType::Private => {
                self.private_vote_cast(wallet, vote_plan, proposal_index, choice)
            }
        }
    }

    pub fn public_vote_cast(
        &self,
        wallet: &Wallet,
        vote_plan: &VotePlan,
        proposal_index: u8,
        choice: &Choice,
    ) -> Fragment {
        let inner_wallet = wallet.clone().into();
        let vote_cast = VoteCast::new(
            vote_plan.to_id(),
            proposal_index as u8,
            Payload::public(*choice),
        );
        self.fragment_factory
            .vote_cast(self.valid_until, &inner_wallet, vote_cast)
    }

    pub fn private_vote_cast(
        &self,
        wallet: &Wallet,
        vote_plan: &VotePlan,
        proposal_index: u8,
        choice: &Choice,
    ) -> Fragment {
        let mut rng = ChaCha20Rng::from_seed([0u8; 32]);

        let election_key =
            chain_vote::ElectionPublicKey::from_participants(vote_plan.committee_public_keys());

        let options = vote_plan
            .proposals()
            .iter()
            .nth((proposal_index).into())
            .unwrap()
            .options();

        let length = options
            .choice_range()
            .end
            .checked_sub(options.choice_range().start)
            .unwrap();

        let choice = choice.as_byte() - options.choice_range().start;
        let vote = chain_vote::Vote::new(length as usize, choice as usize);
        let crs = chain_vote::Crs::from_hash(vote_plan.to_id().as_ref());
        let (encrypted_vote, proof) =
            chain_impl_mockchain::vote::encrypt_vote(&mut rng, &crs, &election_key, vote);

        let vote_cast = VoteCast::new(
            vote_plan.to_id(),
            proposal_index as u8,
            Payload::Private {
                encrypted_vote,
                proof,
            },
        );

        let inner_wallet = wallet.clone().into();

        self.fragment_factory
            .vote_cast(self.valid_until, &inner_wallet, vote_cast)
    }

    pub fn vote_tally(
        &self,
        wallet: &Wallet,
        vote_plan: &VotePlan,
        payload: VoteTallyPayload,
    ) -> Fragment {
        let inner_wallet = wallet.clone().into();

        let vote_tally = match payload {
            VoteTallyPayload::Public => VoteTally::new_public(vote_plan.to_id()),
            VoteTallyPayload::Private { inner } => VoteTally::new_private(vote_plan.to_id(), inner),
        };
        self.fragment_factory
            .vote_tally(self.valid_until, &inner_wallet, vote_tally)
    }

    pub fn update_proposal(
        &self,
        wallet: &Wallet,
        update_proposal: UpdateProposal,
        bft_auth: &SecretKey<Ed25519>,
    ) -> Fragment {
        let inner_wallet = wallet.clone().into();
        let signer_wallet: Wallet = AccountWallet::from_secret_key(
            bft_auth.clone().into(),
            Default::default(),
            wallet.discrimination(),
        )
        .into();

        self.fragment_factory.update_proposal(
            self.valid_until,
            &inner_wallet,
            &signer_wallet.into(),
            update_proposal,
        )
    }

    pub fn update_vote(
        &self,
        wallet: &Wallet,
        update_vote: UpdateVote,
        bft_auth: &SecretKey<Ed25519>,
    ) -> Fragment {
        let inner_wallet = wallet.clone().into();
        let signer_wallet: Wallet = AccountWallet::from_secret_key(
            bft_auth.clone().into(),
            Default::default(),
            wallet.discrimination(),
        )
        .into();
        self.fragment_factory.update_vote(
            self.valid_until,
            &inner_wallet,
            &signer_wallet.into(),
            update_vote,
        )
    }

    #[cfg(feature = "evm")]
    pub fn evm_mapping(&self, from: &Wallet, evm_mapping: &EvmMapping) -> Fragment {
        let inner_wallet = from.clone().into();
        self.fragment_factory
            .evm_mapping(self.valid_until, &inner_wallet, evm_mapping.clone())
    }

    #[cfg(feature = "evm")]
    pub fn evm_transaction(&self, evm_transaction: EvmTransaction) -> Fragment {
        self.fragment_factory.evm_transaction(evm_transaction)
    }
}
