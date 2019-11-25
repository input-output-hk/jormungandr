//! Mockchain ledger. Ledger exists in order to update the
//! current state and verify transactions.

use super::check::{self, TxVerifyError};
use super::pots::Pots;
use crate::block::{ConsensusVersion, LeadersParticipationRecord};
use crate::config::{self, ConfigParam};
use crate::fee::{FeeAlgorithm, LinearFee};
use crate::fragment::{Fragment, FragmentId};
use crate::header::{BlockDate, ChainLength, HeaderContentEvalContext, HeaderId};
use crate::leadership::genesis::ActiveSlotsCoeffError;
use crate::rewards;
use crate::stake::{PoolError, PoolStakeInformation, PoolsState, StakeDistribution};
use crate::transaction::*;
use crate::treasury::Treasury;
use crate::value::*;
use crate::{account, certificate, legacy, multisig, setting, stake, update, utxo};
use chain_addr::{Address, Discrimination, Kind};
use chain_crypto::Verification;
use chain_time::{Epoch, SlotDuration, TimeEra, TimeFrame, Timeline};
use std::mem::swap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

// static parameters, effectively this is constant in the parameter of the blockchain
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerStaticParameters {
    pub block0_initial_hash: HeaderId,
    pub block0_start_time: config::Block0Date,
    pub discrimination: Discrimination,
    pub kes_update_speed: u32,
}

// parameters to validate ledger
#[derive(Debug, Clone)]
pub struct LedgerParameters {
    pub fees: LinearFee,
    pub reward_params: rewards::Parameters,
}

/// Overall ledger structure.
///
/// This represent a given state related to utxo/old utxo/accounts/... at a given
/// point in time.
///
/// The ledger can be easily and cheaply cloned despite containing reference
/// to a lot of data (millions of utxos, thousands of accounts, ..)
#[derive(Clone, PartialEq, Eq)]
pub struct Ledger {
    pub(crate) utxos: utxo::Ledger<Address>,
    pub(crate) oldutxos: utxo::Ledger<legacy::OldAddress>,
    pub(crate) accounts: account::Ledger,
    pub(crate) settings: setting::Settings,
    pub(crate) updates: update::UpdateState,
    pub(crate) multisig: multisig::Ledger,
    pub(crate) delegation: PoolsState,
    pub(crate) static_params: Arc<LedgerStaticParameters>,
    pub(crate) date: BlockDate,
    pub(crate) chain_length: ChainLength,
    pub(crate) era: TimeEra,
    pub(crate) pots: Pots,
    pub(crate) leaders_log: LeadersParticipationRecord,
}

custom_error! {
    #[derive(Clone, PartialEq, Eq)]
    pub Block0Error
        TransactionHasInput = "Transaction should not have inputs in a block0",
        CertTransactionHasInput = "Certificate should not have inputs in a block0",
        CertTransactionHasOutput = "Certificate should not have outputs in a block0",
        TransactionHasWitnesses = "Transaction should not have witnesses in a block0",
        InitialMessageMissing = "The initial message is missing.",
        InitialMessageMany = "Only one initial message is required",
        InitialMessageDuplicateBlock0Date = "Block0 Date is duplicated in the initial message",
        InitialMessageDuplicateDiscrimination = "Address discrimination setting is duplicated in the initial fragment",
        InitialMessageDuplicateConsensusVersion = "Consensus version is duplicated in the initial fragment",
        InitialMessageDuplicateSlotDuration = "Slot Duration is duplicated in the initial fragment",
        InitialMessageDuplicateEpochStabilityDepth = "Epoch stability depth is duplicated in the initial fragment",
        InitialMessageDuplicatePraosActiveSlotsCoeff = "Praos active slot coefficient setting is duplicated in the initial fragment",
        InitialMessageNoDate = "Missing block0 date in the initial fragment",
        InitialMessageNoSlotDuration = "Missing slot duration in the initial fragment",
        InitialMessageNoSlotsPerEpoch = "Missing slots per epoch in the initial fragment",
        InitialMessageNoDiscrimination = "Missing address discrimination in the initial fragment",
        InitialMessageNoConsensusVersion = "Missing consensus version in the initial fragment",
        InitialMessageNoConsensusLeaderId = "Missing consensus leader id list in the initial fragment",
        InitialMessageNoPraosActiveSlotsCoeff = "Missing praos active slot coefficient in the initial fragment",
        InitialMessageNoKesUpdateSpeed = "Missing KES Update speed in the initial fragment",
        UtxoTotalValueTooBig = "Total initial value is too big",
        HasOwnerStakeDelegation = "Owner stake delegation are not valid in the block0",
        HasUpdateProposal = "Update proposal fragments are not valid in the block0",
        HasUpdateVote = "Update vote fragments are not valid in the block0",
        HasPoolManagement = "Pool management are not valid in the block0",
}

pub type OutputOldAddress = Output<legacy::OldAddress>;
pub type OutputAddress = Output<Address>;

custom_error! {
    #[derive(Clone, PartialEq, Eq)]
    pub Error
        Config { source: config::Error } = "Invalid settings",
        UtxoValueNotMatching { expected: Value, value: Value } = "The UTxO value ({expected}) in the transaction does not match the actually state value: {value}",
        UtxoError { source: utxo::Error } = "Invalid UTxO",
        UtxoInvalidSignature { utxo: UtxoPointer, output: OutputAddress, witness: Witness } = "Transaction with invalid signature",
        OldUtxoInvalidSignature { utxo: UtxoPointer, output: OutputOldAddress, witness: Witness } = "Old Transaction with invalid signature",
        OldUtxoInvalidPublicKey { utxo: UtxoPointer, output: OutputOldAddress, witness: Witness } = "Old Transaction with invalid public key",
        AccountInvalidSignature { account: account::Identifier, witness: Witness } = "Account with invalid signature",
        MultisigInvalidSignature { multisig: multisig::Identifier, witness: Witness } = "Multisig with invalid signature",
        TransactionMalformed { source: TxVerifyError } = "Transaction malformed",
        FeeCalculationError { source: ValueError } = "Error while computing the fees",
        PraosActiveSlotsCoeffInvalid { error: ActiveSlotsCoeffError } = "Praos active slot coefficient invalid: {error}",
        TransactionBalanceInvalid { source: BalanceError } = "Failed to validate transaction balance",
        Block0 { source: Block0Error } = "Invalid Block0",
        Block0OnlyFragmentReceived = "Old UTxOs and Initial Message are not valid in a normal block",
        Account { source: account::LedgerError } = "Error or Invalid account",
        Multisig { source: multisig::LedgerError } = "Error or Invalid multisig",
        NotBalanced { inputs: Value, outputs: Value } = "Inputs, outputs and fees are not balanced, transaction with {inputs} input and {outputs} output",
        ZeroOutput { output: Output<Address> } = "Empty output",
        OutputGroupInvalid { output: Output<Address> } = "Output group invalid",
        Delegation { source: PoolError } = "Error or Invalid delegation",
        AccountIdentifierInvalid = "Invalid account identifier",
        InvalidDiscrimination = "Invalid discrimination",
        ExpectingAccountWitness = "Expected an account witness",
        ExpectingUtxoWitness = "Expected a UTxO witness",
        ExpectingInitialMessage = "Expected an Initial Fragment",
        CertificateInvalidSignature = "Invalid certificate's signature",
        Update { source: update::Error } = "Error or Invalid update",
        OwnerStakeDelegationInvalidTransaction = "Transaction for OwnerStakeDelegation is invalid. expecting 1 input, 1 witness and 0 output",
        WrongChainLength { actual: ChainLength, expected: ChainLength } = "Wrong chain length, expected {expected} but received {actual}",
        NonMonotonicDate { block_date: BlockDate, chain_date: BlockDate } = "Non Monotonic date, chain date is at {chain_date} but the block is at {block_date}",
        IncompleteLedger = "Ledger cannot be reconstructed from serialized state because of missing entries",
        PotValueInvalid { error: ValueError } = "Ledger pot value invalid: {error}",
        PoolRegistrationHasNoOwner = "Pool registration with no owner",
        PoolRegistrationHasTooManyOwners = "Pool registration with too many owners",
        PoolRegistrationHasTooManyOperators = "Pool registration with too many operators",
        PoolRegistrationManagementThresholdZero = "Pool registration management threshold is zero",
        PoolRegistrationManagementThresholdAbove = "Pool registration management threshold above owners",
        PoolUpdateNotAllowedYet = "Pool Update not allowed yet",
        StakeDelegationSignatureFailed = "Stake Delegation payload signature failed",
        PoolRetirementSignatureFailed = "Pool Retirement payload signature failed",
        PoolUpdateSignatureFailed = "Pool update payload signature failed",
        UpdateNotAllowedYet = "Update not yet allowed",
}

impl Ledger {
    fn empty(
        settings: setting::Settings,
        static_params: LedgerStaticParameters,
        era: TimeEra,
        pots: Pots,
    ) -> Self {
        Ledger {
            utxos: utxo::Ledger::new(),
            oldutxos: utxo::Ledger::new(),
            accounts: account::Ledger::new(),
            settings,
            updates: update::UpdateState::new(),
            multisig: multisig::Ledger::new(),
            delegation: PoolsState::new(),
            static_params: Arc::new(static_params),
            date: BlockDate::first(),
            chain_length: ChainLength(0),
            era,
            pots,
            leaders_log: LeadersParticipationRecord::new(),
        }
    }

    pub fn new<'a, I>(block0_initial_hash: HeaderId, contents: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = &'a Fragment>,
    {
        let mut content_iter = contents.into_iter();

        let init_ents = match content_iter.next() {
            Some(Fragment::Initial(ref init_ents)) => Ok(init_ents),
            Some(_) => Err(Error::ExpectingInitialMessage),
            None => Err(Error::Block0 {
                source: Block0Error::InitialMessageMissing,
            }),
        }?;

        let mut ledger = {
            let mut regular_ents = crate::fragment::ConfigParams::new();
            let mut block0_start_time = None;
            let mut slot_duration = None;
            let mut discrimination = None;
            let mut slots_per_epoch = None;
            let mut kes_update_speed = None;
            let mut pots = Pots::zero();

            for param in init_ents.iter() {
                match param {
                    ConfigParam::Block0Date(d) => {
                        block0_start_time = Some(*d);
                    }
                    ConfigParam::Discrimination(d) => {
                        discrimination = Some(*d);
                    }
                    ConfigParam::SlotDuration(d) => {
                        slot_duration = Some(*d);
                    }
                    ConfigParam::SlotsPerEpoch(n) => {
                        slots_per_epoch = Some(*n);
                    }
                    ConfigParam::KESUpdateSpeed(n) => {
                        kes_update_speed = Some(*n);
                    }
                    ConfigParam::TreasuryAdd(v) => {
                        pots.treasury = Treasury::initial(*v);
                    }
                    ConfigParam::RewardPot(v) => {
                        pots.rewards = *v;
                    }
                    _ => regular_ents.push(param.clone()),
                }
            }

            // here we make sure those specific parameters are present, otherwise we returns a given error
            let block0_start_time = block0_start_time.ok_or(Error::Block0 {
                source: Block0Error::InitialMessageNoDate,
            })?;
            let discrimination = discrimination.ok_or(Error::Block0 {
                source: Block0Error::InitialMessageNoDiscrimination,
            })?;
            let slot_duration = slot_duration.ok_or(Error::Block0 {
                source: Block0Error::InitialMessageNoSlotDuration,
            })?;
            let slots_per_epoch = slots_per_epoch.ok_or(Error::Block0 {
                source: Block0Error::InitialMessageNoSlotsPerEpoch,
            })?;
            let kes_update_speed = kes_update_speed.ok_or(Error::Block0 {
                source: Block0Error::InitialMessageNoKesUpdateSpeed,
            })?;

            let static_params = LedgerStaticParameters {
                block0_initial_hash,
                block0_start_time: block0_start_time,
                discrimination: discrimination,
                kes_update_speed: kes_update_speed,
            };

            let system_time = SystemTime::UNIX_EPOCH + Duration::from_secs(block0_start_time.0);
            let timeline = Timeline::new(system_time);
            let tf = TimeFrame::new(timeline, SlotDuration::from_secs(slot_duration as u32));
            let slot0 = tf.slot0();

            let era = TimeEra::new(slot0, Epoch(0), slots_per_epoch);

            let settings = setting::Settings::new().apply(&regular_ents)?;

            if settings.bft_leaders.is_empty() {
                return Err(Error::Block0 {
                    source: Block0Error::InitialMessageNoConsensusLeaderId,
                });
            }
            Ledger::empty(settings, static_params, era, pots)
        };

        for content in content_iter {
            let fragment_id = content.hash();
            match content {
                Fragment::Initial(_) => {
                    return Err(Error::Block0 {
                        source: Block0Error::InitialMessageMany,
                    });
                }
                Fragment::OldUtxoDeclaration(old) => {
                    ledger.oldutxos = apply_old_declaration(&fragment_id, ledger.oldutxos, old)?;
                }
                Fragment::Transaction(tx) => {
                    let tx = tx.as_slice();
                    check::valid_block0_transaction_no_inputs(&tx)?;

                    ledger = ledger.apply_tx_outputs(fragment_id, tx.outputs())?;
                }
                Fragment::UpdateProposal(_) => {
                    return Err(Error::Block0 {
                        source: Block0Error::HasUpdateProposal,
                    });
                }
                Fragment::UpdateVote(_) => {
                    return Err(Error::Block0 {
                        source: Block0Error::HasUpdateVote,
                    });
                }
                Fragment::OwnerStakeDelegation(_) => {
                    return Err(Error::Block0 {
                        source: Block0Error::HasOwnerStakeDelegation,
                    });
                }
                Fragment::StakeDelegation(tx) => {
                    let tx = tx.as_slice();
                    check::valid_block0_cert_transaction(&tx)?;
                    ledger = ledger.apply_stake_delegation(&tx.payload().into_payload())?;
                }
                Fragment::PoolRegistration(tx) => {
                    let tx = tx.as_slice();
                    check::valid_block0_cert_transaction(&tx)?;
                    ledger = ledger.apply_pool_registration(&tx.payload().into_payload())?;
                }
                Fragment::PoolRetirement(_) => {
                    return Err(Error::Block0 {
                        source: Block0Error::HasPoolManagement,
                    });
                }
                Fragment::PoolUpdate(_) => {
                    return Err(Error::Block0 {
                        source: Block0Error::HasPoolManagement,
                    });
                }
            }
        }

        ledger.validate_utxo_total_value()?;
        Ok(ledger)
    }

    /// This need to be called before the *first* block of a new epoch
    ///
    /// * Reset the leaders log
    pub fn distribute_rewards<'a>(
        &'a self,
        distribution: &StakeDistribution,
        ledger_params: &LedgerParameters,
    ) -> Result<Self, Error> {
        let mut new_ledger = self.clone();

        // grab the total contribution in the system
        // with all the stake pools and start rewarding them

        let expected_epoch_reward = rewards::rewards_contribution_calculation(
            new_ledger.date.epoch + 1,
            &ledger_params.reward_params,
        );

        let mut total_reward = new_ledger.pots.draw_reward(expected_epoch_reward);

        // Move fees in the rewarding pots for distribution or depending on settings
        // to the treasury directly
        if true {
            total_reward = (total_reward + new_ledger.pots.siphon_fees()).unwrap();
        } else {
            let fees = new_ledger.pots.siphon_fees();
            new_ledger.pots.treasury_add(fees)?
        }

        let mut leaders_log = LeadersParticipationRecord::new();
        swap(&mut new_ledger.leaders_log, &mut leaders_log);

        let total_blocks = leaders_log.total();
        let reward_unit = total_reward.split_in(total_blocks);

        for (pool_id, pool_blocks) in leaders_log.iter() {
            let pool_total_reward = reward_unit.parts.scale(*pool_blocks).unwrap();

            match (
                new_ledger
                    .delegation
                    .stake_pool_get(&pool_id)
                    .map(|reg| reg.clone()),
                distribution.to_pools.get(pool_id),
            ) {
                (Ok(pool_reg), Some(pool_distribution)) => {
                    new_ledger.distribute_poolid_rewards(
                        &pool_reg,
                        pool_total_reward,
                        pool_distribution,
                    )?;
                }
                _ => {
                    // dump reward to treasury
                    new_ledger.pots.treasury_add(pool_total_reward)?;
                }
            }
        }

        if reward_unit.remaining > Value::zero() {
            // if anything remaining, put it in treasury
            new_ledger.pots.treasury_add(reward_unit.remaining)?;
        }

        Ok(new_ledger)
    }

    fn distribute_poolid_rewards(
        &mut self,
        reg: &certificate::PoolRegistration,
        total_reward: Value,
        distribution: &PoolStakeInformation,
    ) -> Result<(), Error> {
        let distr = rewards::tax_cut(total_reward, &reg.rewards).unwrap();

        // distribute to pool owners (or the reward account)
        match &reg.reward_account {
            Some(reward_account) => match reward_account {
                AccountIdentifier::Single(single_account) => {
                    self.add_value_or_create_account(&single_account, distr.taxed)?;
                }
                AccountIdentifier::Multi(_multi_account) => unimplemented!(),
            },
            None => panic!("rewards to owners not implemented"),
        }

        // distribute the rest to delegators
        let total = distribution.total;
        for (account, stake) in distribution.stake_owners.iter() {
            let r = Value(1);
            self.add_value_or_create_account(account, r)?;
        }
        Ok(())
    }

    /// Try to apply messages to a State, and return the new State if succesful
    pub fn apply_block<'a, I>(
        &'a self,
        ledger_params: &LedgerParameters,
        contents: I,
        metadata: &HeaderContentEvalContext,
    ) -> Result<Self, Error>
    where
        I: IntoIterator<Item = &'a Fragment>,
    {
        let mut new_ledger = self.clone();

        new_ledger.chain_length = self.chain_length.increase();

        // Check if the metadata (date/heigth) check out compared to the current state
        if metadata.chain_length != new_ledger.chain_length {
            return Err(Error::WrongChainLength {
                actual: metadata.chain_length,
                expected: new_ledger.chain_length,
            });
        }

        if metadata.block_date <= new_ledger.date {
            return Err(Error::NonMonotonicDate {
                block_date: metadata.block_date,
                chain_date: new_ledger.date,
            });
        }

        // double check that if we had an epoch transition, distribute_rewards has been called
        if metadata.block_date.epoch > new_ledger.date.epoch {
            if self.leaders_log.total() > 0 {
                panic!("internal error: apply_block called after epoch transition, but distribute_rewards has not been called")
            }
        }

        // Process Update proposals if needed
        let (updates, settings) = new_ledger.updates.process_proposals(
            new_ledger.settings,
            new_ledger.date,
            metadata.block_date,
        )?;
        new_ledger.updates = updates;
        new_ledger.settings = settings;

        // Apply all the fragments
        for content in contents {
            new_ledger = new_ledger.apply_fragment(ledger_params, content, metadata.block_date)?;
        }

        // Update the ledger metadata related to eval context
        new_ledger.date = metadata.block_date;
        match metadata.gp_content {
            None => {}
            Some(ref gp_content) => {
                new_ledger
                    .settings
                    .consensus_nonce
                    .hash_with(&gp_content.nonce);
                new_ledger
                    .leaders_log
                    .increase_for(&gp_content.pool_creator);
            }
        };

        Ok(new_ledger)
    }

    /// Try to apply a message to the State, and return the new State if successful
    ///
    /// this does not _advance_ the state to the new _state_ but apply a simple fragment
    /// of block to the current context.
    ///
    pub fn apply_fragment(
        &self,
        ledger_params: &LedgerParameters,
        content: &Fragment,
        block_date: BlockDate,
    ) -> Result<Self, Error> {
        let mut new_ledger = self.clone();

        let fragment_id = content.hash();
        match content {
            Fragment::Initial(_) => return Err(Error::Block0OnlyFragmentReceived),
            Fragment::OldUtxoDeclaration(_) => return Err(Error::Block0OnlyFragmentReceived),
            Fragment::Transaction(tx) => {
                let tx = tx.as_slice();
                let (new_ledger_, _fee) =
                    new_ledger.apply_transaction(&fragment_id, &tx, &ledger_params)?;
                new_ledger = new_ledger_;
            }
            Fragment::OwnerStakeDelegation(tx) => {
                let tx = tx.as_slice();
                let (new_ledger_, _fee) =
                    new_ledger.apply_owner_stake_delegation(&tx, &ledger_params)?;
                new_ledger = new_ledger_;
            }
            Fragment::StakeDelegation(tx) => {
                let tx = tx.as_slice();
                let payload = tx.payload().into_payload();
                let payload_auth = tx.payload_auth().into_payload_auth();
                let verified = match payload_auth {
                    AccountBindingSignature::Single(signature) => {
                        let account_pk = payload
                            .account_id
                            .to_single_account()
                            .ok_or(Error::AccountIdentifierInvalid)?;
                        signature
                            .verify_slice(&account_pk.into(), &tx.transaction_binding_auth_data())
                    }
                    AccountBindingSignature::Multi(_) => {
                        // TODO
                        Verification::Failed
                    }
                };

                if verified == Verification::Failed {
                    return Err(Error::StakeDelegationSignatureFailed);
                }

                let (new_ledger_, _fee) =
                    new_ledger.apply_transaction(&fragment_id, &tx, &ledger_params)?;
                new_ledger = new_ledger_.apply_stake_delegation(&payload)?;
            }
            Fragment::PoolRegistration(tx) => {
                let tx = tx.as_slice();
                let (new_ledger_, _fee) =
                    new_ledger.apply_transaction(&fragment_id, &tx, &ledger_params)?;
                new_ledger = new_ledger_.apply_pool_registration_signcheck(
                    &tx.payload().into_payload(),
                    &tx.transaction_binding_auth_data(),
                    tx.payload_auth().into_payload_auth(),
                )?;
            }
            Fragment::PoolRetirement(tx) => {
                let tx = tx.as_slice();

                let (new_ledger_, _fee) =
                    new_ledger.apply_transaction(&fragment_id, &tx, &ledger_params)?;
                new_ledger = new_ledger_.apply_pool_retirement(
                    &tx.payload().into_payload(),
                    &tx.transaction_binding_auth_data(),
                    tx.payload_auth().into_payload_auth(),
                )?;
            }
            Fragment::PoolUpdate(tx) => {
                let tx = tx.as_slice();

                let (new_ledger_, _fee) =
                    new_ledger.apply_transaction(&fragment_id, &tx, &ledger_params)?;
                new_ledger = new_ledger_.apply_pool_update(
                    &tx.payload().into_payload(),
                    &tx.transaction_binding_auth_data(),
                    tx.payload_auth().into_payload_auth(),
                )?;
            }
            Fragment::UpdateProposal(update_proposal) => {
                if true {
                    return Err(Error::UpdateNotAllowedYet);
                }
                new_ledger =
                    new_ledger.apply_update_proposal(fragment_id, &update_proposal, block_date)?;
            }
            Fragment::UpdateVote(vote) => {
                if true {
                    return Err(Error::UpdateNotAllowedYet);
                }
                new_ledger = new_ledger.apply_update_vote(&vote)?;
            }
        }

        Ok(new_ledger)
    }

    pub fn apply_transaction<'a, Extra>(
        mut self,
        fragment_id: &FragmentId,
        tx: &TransactionSlice<'a, Extra>,
        dyn_params: &LedgerParameters,
    ) -> Result<(Self, Value), Error>
    where
        Extra: Payload,
        LinearFee: FeeAlgorithm,
    {
        check::valid_transaction_ios_number(tx)?;
        let fee = calculate_fee(tx, dyn_params);
        tx.into_owned().verify_strictly_balanced(fee)?;
        self = self.apply_tx_inputs(tx)?;
        self = self.apply_tx_outputs(*fragment_id, tx.outputs())?;
        self = self.apply_tx_fee(fee)?;
        Ok((self, fee))
    }

    pub fn apply_update(mut self, update: &update::UpdateProposal) -> Result<Self, Error> {
        self.settings = self.settings.apply(&update.changes)?;
        Ok(self)
    }

    pub fn apply_update_proposal(
        mut self,
        proposal_id: update::UpdateProposalId,
        proposal: &update::SignedUpdateProposal,
        cur_date: BlockDate,
    ) -> Result<Self, Error> {
        self.updates =
            self.updates
                .apply_proposal(proposal_id, proposal, &self.settings, cur_date)?;
        Ok(self)
    }

    pub fn apply_update_vote(mut self, vote: &update::SignedUpdateVote) -> Result<Self, Error> {
        self.updates = self.updates.apply_vote(vote, &self.settings)?;
        Ok(self)
    }

    pub fn apply_pool_registration_signcheck<'a>(
        self,
        cert: &certificate::PoolRegistration,
        bad: &TransactionBindingAuthData<'a>,
        sig: certificate::PoolSignature,
    ) -> Result<Self, Error> {
        check::valid_pool_registration_certificate(cert)?;
        check::valid_pool_signature(&sig)?;

        if sig.verify(cert, bad) == Verification::Failed {
            return Err(Error::PoolRetirementSignatureFailed);
        }

        self.apply_pool_registration(cert)
    }

    pub fn apply_pool_registration(
        mut self,
        cert: &certificate::PoolRegistration,
    ) -> Result<Self, Error> {
        check::valid_pool_registration_certificate(cert)?;

        self.delegation = self.delegation.register_stake_pool(cert.clone())?;
        Ok(self)
    }

    pub fn apply_pool_retirement<'a>(
        mut self,
        auth_cert: &certificate::PoolRetirement,
        bad: &TransactionBindingAuthData<'a>,
        sig: certificate::PoolSignature,
    ) -> Result<Self, Error> {
        check::valid_pool_retirement_certificate(auth_cert)?;
        check::valid_pool_signature(&sig)?;

        let reg = self.delegation.stake_pool_get(&auth_cert.pool_id)?;
        if sig.verify(reg, bad) == Verification::Failed {
            return Err(Error::PoolRetirementSignatureFailed);
        }

        self.delegation = self.delegation.deregister_stake_pool(&auth_cert.pool_id)?;
        Ok(self)
    }

    pub fn apply_pool_update<'a>(
        self,
        auth_cert: &certificate::PoolUpdate,
        bad: &TransactionBindingAuthData<'a>,
        sig: certificate::PoolSignature,
    ) -> Result<Self, Error> {
        check::valid_pool_update_certificate(auth_cert)?;
        check::valid_pool_signature(&sig)?;

        let reg = self.delegation.stake_pool_get(&auth_cert.pool_id)?;
        if sig.verify(reg, bad) == Verification::Failed {
            return Err(Error::PoolUpdateSignatureFailed);
        }
        // TODO do things
        Err(Error::PoolUpdateNotAllowedYet)
    }

    pub fn apply_stake_delegation(
        mut self,
        auth_cert: &certificate::StakeDelegation,
    ) -> Result<Self, Error> {
        let delegation = &auth_cert.delegation;

        let account_key = auth_cert
            .account_id
            .to_single_account()
            .ok_or(Error::AccountIdentifierInvalid)?;
        self.accounts = self.accounts.set_delegation(&account_key, delegation)?;
        Ok(self)
    }

    pub fn apply_owner_stake_delegation<'a>(
        mut self,
        tx: &TransactionSlice<'a, certificate::OwnerStakeDelegation>,
        dyn_params: &LedgerParameters,
    ) -> Result<(Self, Value), Error> {
        let sign_data_hash = tx.transaction_sign_data_hash();

        let (account_id, value, witness) = {
            check::valid_stake_owner_delegation_transaction(tx)?;

            let input = tx.inputs().iter().nth(0).unwrap();
            match input.to_enum() {
                InputEnum::UtxoInput(_) => {
                    return Err(Error::OwnerStakeDelegationInvalidTransaction);
                }
                InputEnum::AccountInput(account_id, value) => {
                    let witness = tx.witnesses().iter().nth(0).unwrap();
                    (account_id, value, witness)
                }
            }
        };

        let fee = dyn_params.fees.calculate_tx(&tx.into_owned());
        if fee != value {
            return Err(Error::NotBalanced {
                inputs: value,
                outputs: fee,
            });
        }

        match match_identifier_witness(&account_id, &witness)? {
            MatchingIdentifierWitness::Single(account_id, witness) => {
                let single = input_single_account_verify(
                    self.accounts,
                    &self.static_params.block0_initial_hash,
                    &sign_data_hash,
                    &account_id,
                    witness,
                    value,
                )?;
                self.accounts = single.set_delegation(
                    &account_id,
                    tx.payload().into_payload().get_delegation_type(),
                )?;
            }
            MatchingIdentifierWitness::Multi(account_id, witness) => {
                let multi = input_multi_account_verify(
                    self.multisig,
                    &self.static_params.block0_initial_hash,
                    &sign_data_hash,
                    &account_id,
                    witness,
                    value,
                )?;
                self.multisig = multi.set_delegation(
                    &account_id,
                    tx.payload().into_payload().get_delegation_type(),
                )?;
            }
        }
        Ok((self, value))
    }

    pub fn get_stake_distribution(&self) -> StakeDistribution {
        stake::get_distribution(&self.accounts, &self.delegation, &self.utxos)
    }

    /// access the ledger static parameters
    pub fn get_static_parameters(&self) -> &LedgerStaticParameters {
        self.static_params.as_ref()
    }

    pub fn accounts(&self) -> &account::Ledger {
        &self.accounts
    }

    pub fn get_ledger_parameters(&self) -> LedgerParameters {
        LedgerParameters {
            fees: *self.settings.linear_fees,
            reward_params: self.settings.to_reward_params(),
        }
    }

    pub fn consensus_version(&self) -> ConsensusVersion {
        self.settings.consensus_version
    }

    pub fn utxo_out(
        &self,
        fragment_id: FragmentId,
        index: TransactionIndex,
    ) -> Option<&Output<Address>> {
        self.utxos
            .get(&fragment_id, &index)
            .map(|entry| entry.output)
    }

    pub fn utxos<'a>(&'a self) -> utxo::Iter<'a, Address> {
        self.utxos.iter()
    }

    pub fn chain_length(&self) -> ChainLength {
        self.chain_length
    }

    pub fn settings(&mut self) -> &mut setting::Settings {
        &mut self.settings
    }

    pub fn delegation(&self) -> &PoolsState {
        &self.delegation
    }

    pub fn delegation_mut(&mut self) -> &mut PoolsState {
        &mut self.delegation
    }

    pub fn date(&self) -> BlockDate {
        self.date
    }

    pub fn era(&self) -> &TimeEra {
        &self.era
    }

    fn validate_utxo_total_value(&self) -> Result<(), Error> {
        let old_utxo_values = self.oldutxos.iter().map(|entry| entry.output.value);
        let new_utxo_values = self.utxos.iter().map(|entry| entry.output.value);
        let account_value = self.accounts.get_total_value().map_err(|_| Error::Block0 {
            source: Block0Error::UtxoTotalValueTooBig,
        })?;
        let multisig_value = self.multisig.get_total_value().map_err(|_| Error::Block0 {
            source: Block0Error::UtxoTotalValueTooBig,
        })?;
        let all_utxo_values = old_utxo_values
            .chain(new_utxo_values)
            .chain(Some(account_value))
            .chain(Some(multisig_value))
            .chain(self.pots.values());
        Value::sum(all_utxo_values).map_err(|_| Error::Block0 {
            source: Block0Error::UtxoTotalValueTooBig,
        })?;
        Ok(())
    }

    fn apply_tx_inputs<'a, Extra: Payload>(
        mut self,
        tx: &TransactionSlice<'a, Extra>,
    ) -> Result<Self, Error> {
        let sign_data_hash = tx.transaction_sign_data_hash();
        for (input, witness) in tx.inputs_and_witnesses().iter() {
            match input.to_enum() {
                InputEnum::UtxoInput(utxo) => {
                    self = self.apply_input_to_utxo(&sign_data_hash, &utxo, &witness)?
                }
                InputEnum::AccountInput(account_id, value) => {
                    match match_identifier_witness(&account_id, &witness)? {
                        MatchingIdentifierWitness::Single(account_id, witness) => {
                            self.accounts = input_single_account_verify(
                                self.accounts,
                                &self.static_params.block0_initial_hash,
                                &sign_data_hash,
                                &account_id,
                                witness,
                                value,
                            )?
                        }
                        MatchingIdentifierWitness::Multi(account_id, witness) => {
                            self.multisig = input_multi_account_verify(
                                self.multisig,
                                &self.static_params.block0_initial_hash,
                                &sign_data_hash,
                                &account_id,
                                witness,
                                value,
                            )?
                        }
                    }
                }
            }
        }
        Ok(self)
    }

    fn apply_tx_outputs<'a>(
        mut self,
        fragment_id: FragmentId,
        outputs: OutputsSlice<'a>,
    ) -> Result<Self, Error> {
        let mut new_utxos = Vec::new();
        for (index, output) in outputs.iter().enumerate() {
            check::valid_output_value(&output)?;

            if output.address.discrimination() != self.static_params.discrimination {
                return Err(Error::InvalidDiscrimination);
            }
            match output.address.kind() {
                Kind::Single(_) => {
                    new_utxos.push((index as u8, output.clone()));
                }
                Kind::Group(_, account_id) => {
                    let account_id = account_id.clone().into();
                    // TODO: probably faster to just call add_account and check for already exists error
                    if !self.accounts.exists(&account_id) {
                        self.accounts =
                            self.accounts.add_account(&account_id, Value::zero(), ())?;
                    }
                    new_utxos.push((index as u8, output.clone()));
                }
                Kind::Account(identifier) => {
                    // don't have a way to make a newtype ref from the ref so .clone()
                    let account = identifier.clone().into();
                    self.add_value_or_create_account(&account, output.value)?;
                }
                Kind::Multisig(identifier) => {
                    let identifier = multisig::Identifier::from(identifier.clone());
                    self.multisig = self.multisig.add_value(&identifier, output.value)?;
                }
            }
        }
        if new_utxos.len() > 0 {
            self.utxos = self.utxos.add(&fragment_id, &new_utxos)?;
        }
        Ok(self)
    }

    fn add_value_or_create_account(
        &mut self,
        account: &account::Identifier,
        value: Value,
    ) -> Result<(), Error> {
        self.accounts = match self.accounts.add_value(account, value) {
            Ok(accounts) => accounts,
            Err(account::LedgerError::NonExistent) => {
                self.accounts.add_account(account, value, ())?
            }
            Err(error) => return Err(error.into()),
        };
        Ok(())
    }

    fn apply_tx_fee(mut self, fee: Value) -> Result<Self, Error> {
        self.pots.append_fees(fee)?;
        Ok(self)
    }

    fn apply_input_to_utxo(
        mut self,
        sign_data_hash: &TransactionSignDataHash,
        utxo: &UtxoPointer,
        witness: &Witness,
    ) -> Result<Self, Error> {
        match witness {
            Witness::Account(_) => Err(Error::ExpectingUtxoWitness),
            Witness::Multisig(_) => Err(Error::ExpectingUtxoWitness),
            Witness::OldUtxo(xpub, signature) => {
                let (old_utxos, associated_output) = self
                    .oldutxos
                    .remove(&utxo.transaction_id, utxo.output_index)?;

                self.oldutxos = old_utxos;
                if utxo.value != associated_output.value {
                    return Err(Error::UtxoValueNotMatching {
                        expected: utxo.value,
                        value: associated_output.value,
                    });
                };

                if legacy::oldaddress_from_xpub(&associated_output.address, xpub)
                    == legacy::OldAddressMatchXPub::No
                {
                    return Err(Error::OldUtxoInvalidPublicKey {
                        utxo: utxo.clone(),
                        output: associated_output.clone(),
                        witness: witness.clone(),
                    });
                };

                let data_to_verify =
                    WitnessUtxoData::new(&self.static_params.block0_initial_hash, sign_data_hash);
                let verified = signature.verify(&xpub, &data_to_verify);
                if verified == chain_crypto::Verification::Failed {
                    return Err(Error::OldUtxoInvalidSignature {
                        utxo: utxo.clone(),
                        output: associated_output.clone(),
                        witness: witness.clone(),
                    });
                };

                Ok(self)
            }
            Witness::Utxo(signature) => {
                let (new_utxos, associated_output) =
                    self.utxos.remove(&utxo.transaction_id, utxo.output_index)?;
                self.utxos = new_utxos;
                if utxo.value != associated_output.value {
                    return Err(Error::UtxoValueNotMatching {
                        expected: utxo.value,
                        value: associated_output.value,
                    });
                }

                let data_to_verify =
                    WitnessUtxoData::new(&self.static_params.block0_initial_hash, sign_data_hash);
                let verified = signature.verify(
                    &associated_output.address.public_key().unwrap(),
                    &data_to_verify,
                );
                if verified == chain_crypto::Verification::Failed {
                    return Err(Error::UtxoInvalidSignature {
                        utxo: utxo.clone(),
                        output: associated_output.clone(),
                        witness: witness.clone(),
                    });
                };
                Ok(self)
            }
        }
    }
}

fn apply_old_declaration(
    fragment_id: &FragmentId,
    mut utxos: utxo::Ledger<legacy::OldAddress>,
    decl: &legacy::UtxoDeclaration,
) -> Result<utxo::Ledger<legacy::OldAddress>, Error> {
    assert!(decl.addrs.len() < 255);
    let mut outputs = Vec::with_capacity(decl.addrs.len());
    for (i, d) in decl.addrs.iter().enumerate() {
        let output = Output {
            address: d.0.clone(),
            value: d.1,
        };
        outputs.push((i as u8, output))
    }
    utxos = utxos.add(&fragment_id, &outputs)?;
    Ok(utxos)
}

fn calculate_fee<'a, Extra: Payload>(
    tx: &TransactionSlice<'a, Extra>,
    dyn_params: &LedgerParameters,
) -> Value {
    dyn_params.fees.calculate_tx(&tx.into_owned())
}

pub enum MatchingIdentifierWitness<'a> {
    Single(account::Identifier, &'a account::Witness),
    Multi(multisig::Identifier, &'a multisig::Witness),
}

fn match_identifier_witness<'a>(
    account: &UnspecifiedAccountIdentifier,
    witness: &'a Witness,
) -> Result<MatchingIdentifierWitness<'a>, Error> {
    match witness {
        Witness::OldUtxo(_, _) => Err(Error::ExpectingAccountWitness),
        Witness::Utxo(_) => Err(Error::ExpectingAccountWitness),
        Witness::Account(sig) => {
            // refine account to a single account identifier
            let account = account
                .to_single_account()
                .ok_or(Error::AccountIdentifierInvalid)?;
            Ok(MatchingIdentifierWitness::Single(account, sig))
        }
        Witness::Multisig(msignature) => {
            // refine account to a multisig account identifier
            let account = account.to_multi_account();
            Ok(MatchingIdentifierWitness::Multi(account, msignature))
        }
    }
}

fn input_single_account_verify<'a>(
    mut ledger: account::Ledger,
    block0_hash: &HeaderId,
    sign_data_hash: &TransactionSignDataHash,
    account: &account::Identifier,
    witness: &'a account::Witness,
    value: Value,
) -> Result<account::Ledger, Error> {
    // .remove_value() check if there's enough value and if not, returns a Err.
    let (new_ledger, spending_counter) = ledger.remove_value(&account, value)?;
    ledger = new_ledger;

    let tidsc = WitnessAccountData::new(block0_hash, sign_data_hash, &spending_counter);
    let verified = witness.verify(&account.clone().into(), &tidsc);
    if verified == chain_crypto::Verification::Failed {
        return Err(Error::AccountInvalidSignature {
            account: account.clone(),
            witness: Witness::Account(witness.clone()),
        });
    };
    Ok(ledger)
}

fn input_multi_account_verify<'a>(
    mut ledger: multisig::Ledger,
    block0_hash: &HeaderId,
    sign_data_hash: &TransactionSignDataHash,
    account: &multisig::Identifier,
    witness: &'a multisig::Witness,
    value: Value,
) -> Result<multisig::Ledger, Error> {
    // .remove_value() check if there's enough value and if not, returns a Err.
    let (new_ledger, declaration, spending_counter) = ledger.remove_value(&account, value)?;

    let data_to_verify = WitnessMultisigData::new(&block0_hash, sign_data_hash, &spending_counter);
    if witness.verify(declaration, &data_to_verify) != true {
        return Err(Error::MultisigInvalidSignature {
            multisig: account.clone(),
            witness: Witness::Multisig(witness.clone()),
        });
    }
    ledger = new_ledger;
    Ok(ledger)
}

#[cfg(test)]
mod tests {
    /*
    use super::*;
    use crate::{
        account::{Identifier, SpendingCounter},
        accounting::account::account_state::AccountState,
        fee::LinearFee,
        key::Hash,
        multisig,
        setting::Settings,
        testing::{
            address::ArbitraryAddressDataValueVec,
            builders::{witness_builder, TransactionBuilder},
            data::{AddressData, AddressDataValue},
            ledger::{self as ledger_mock, ConfigBuilder},
            TestGen,
        },
        transaction::Witness,
    };
    use chain_addr::Discrimination;
    use quickcheck::{Arbitrary, Gen, TestResult};
    use quickcheck_macros::quickcheck;
    use std::{fmt, iter};

    impl Arbitrary for LedgerStaticParameters {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            LedgerStaticParameters {
                block0_initial_hash: Arbitrary::arbitrary(g),
                block0_start_time: Arbitrary::arbitrary(g),
                discrimination: Arbitrary::arbitrary(g),
                kes_update_speed: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for LedgerParameters {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            LedgerParameters {
                fees: Arbitrary::arbitrary(g),
                reward_params: Arbitrary::arbitrary(g),
            }
        }
    }

    #[derive(Clone)]
    pub struct ArbitraryEmptyLedger(Ledger);

    impl Arbitrary for ArbitraryEmptyLedger {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut ledger = Ledger::empty(
                Arbitrary::arbitrary(g),
                Arbitrary::arbitrary(g),
                Arbitrary::arbitrary(g),
                Arbitrary::arbitrary(g),
            );

            ledger.date = Arbitrary::arbitrary(g);
            ledger.chain_length = Arbitrary::arbitrary(g);
            ArbitraryEmptyLedger(ledger)
        }
    }

    impl fmt::Debug for ArbitraryEmptyLedger {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Ledger")
                .field("chain_length", &self.0.chain_length())
                .field("settings", &self.0.settings)
                .field("date", &self.0.date())
                .field("era", &self.0.era())
                .field("static_params", &self.0.get_static_parameters().clone())
                .finish()
        }
    }

    impl Into<Ledger> for ArbitraryEmptyLedger {
        fn into(self) -> Ledger {
            self.0
        }
    }

    #[quickcheck]
    pub fn apply_empty_block_prop_test(
        context: HeaderContentEvalContext,
        ledger: ArbitraryEmptyLedger,
    ) -> TestResult {
        let ledger: Ledger = ledger.into();
        let should_succeed =
            context.chain_length == ledger.chain_length.next() && context.block_date > ledger.date;

        let result = ledger.apply_block(&ledger.get_ledger_parameters(), Vec::new(), &context);
        match (result, should_succeed) {
            (Ok(_), true) => TestResult::passed(),
            (Ok(_), false) => TestResult::error("should pass"),
            (Err(err), true) => TestResult::error(format!("unexpected error: {}", err)),
            (Err(_), false) => TestResult::passed(),
        }
    }

    #[quickcheck]
    pub fn match_identifier_witness_prop_test(
        id: AccountIdentifier,
        witness: Witness,
    ) -> TestResult {
        let result = super::match_identifier_witness(&id, &witness);
        match (witness.clone(), result) {
            (Witness::OldUtxo(_, _), Ok(_)) => {
                TestResult::error("expecting error, but got success")
            }
            (Witness::OldUtxo(_, _), Err(_)) => TestResult::passed(),
            (Witness::Utxo(_), Ok(_)) => TestResult::error("expecting error, but got success"),
            (Witness::Utxo(_), Err(_)) => TestResult::passed(),
            (Witness::Account(_), Ok(_)) => TestResult::passed(),
            (Witness::Account(_), Err(_)) => TestResult::error("unexpected error"),
            (Witness::Multisig(_), _) => TestResult::discard(),
        }
    }

    /*
    #[quickcheck]
    pub fn input_single_account_verify_negative_prop_test(
        id: Identifier,
        account_state: AccountState<()>,
        value_to_sub: Value,
        block0_hash: &HeaderId,
        sign_data_hash: TransactionSignDataHash,
        witness: account::Witness,
    ) -> TestResult {
        let mut account_ledger = account::Ledger::new();
        account_ledger = account_ledger
            .add_account(&id, account_state.get_value(), ())
            .unwrap();
        let result = super::input_single_account_verify(
            account_ledger,
            &block0_hash,
            &sign_data_hash,
            &id,
            &witness,
            value_to_sub,
        );

        TestResult::from_bool(result.is_err())
    }
    */

    #[test]
    pub fn test_input_single_account_verify_correct_account() {
        let account = AddressData::account(Discrimination::Test);
        let initial_value = Value(100);
        let value_to_sub = Value(80);
        let block0_hash = TestGen::hash();
        let id: Identifier = account.public_key().into();

        let account_ledger = account_ledger_with_initials(&[(id.clone(), initial_value)]);
        let signed_tx = create_empty_transaction(&block0_hash, &account);
        let sign_data_hash = signed_tx.hash();

        let result = super::input_single_account_verify(
            account_ledger,
            &block0_hash,
            &sign_data_hash,
            &id,
            &to_account_witness(signed_tx.witnesses().iter().next().unwrap()),
            value_to_sub,
        );
        assert!(result.is_ok())
    }

    fn create_empty_transaction(
        block0_hash: &HeaderId,
        address_data: &AddressData,
    ) -> Transaction<NoExtra> {
        TransactionBuilder::new()
            .authenticate()
            .with_witness(&block0_hash, &address_data)
            .seal()
    }

    fn account_ledger_with_initials(initials: &[(Identifier, Value)]) -> account::Ledger {
        let mut account_ledger = account::Ledger::new();
        for (id, initial_value) in initials {
            account_ledger = account_ledger
                .add_account(&id, initial_value.clone(), ())
                .unwrap();
        }
        account_ledger
    }

    #[test]
    pub fn test_input_single_account_verify_different_block0_hash() {
        let account = AddressData::account(Discrimination::Test);
        let initial_value = Value(100);
        let value_to_sub = Value(80);
        let block0_hash = TestGen::hash();
        let wrong_block0_hash = TestGen::hash();
        let id: Identifier = account.public_key().into();

        let account_ledger = account_ledger_with_initials(&[(id.clone(), initial_value)]);
        let signed_tx = create_empty_transaction(&block0_hash, &account);
        let sign_data_hash = signed_tx.transaction.hash();

        let result = super::input_single_account_verify(
            account_ledger,
            &wrong_block0_hash,
            &sign_data_hash,
            &id,
            &to_account_witness(signed_tx.witnesses.iter().next().unwrap()),
            value_to_sub,
        );
        assert!(result.is_err())
    }

    fn to_account_witness(witness: &Witness) -> &account::Witness {
        match witness {
            Witness::Account(account_witness) => account_witness,
            _ => panic!("wrong type of witness"),
        }
    }

    #[test]
    pub fn test_input_account_wrong_value() {
        let account = AddressData::account(Discrimination::Test);
        let initial_value = Value(100);
        let value_to_sub = Value(110);
        let block0_hash = TestGen::hash();
        let wrong_block0_hash = TestGen::hash();
        let id: Identifier = account.public_key().into();

        let account_ledger = account_ledger_with_initials(&[(id.clone(), initial_value)]);
        let signed_tx = create_empty_transaction(&block0_hash, &account);
        let sign_data_hash = signed_tx.transaction.hash();

        let result = super::input_single_account_verify(
            account_ledger,
            &wrong_block0_hash,
            &sign_data_hash,
            &id,
            &to_account_witness(signed_tx.witnesses.iter().next().unwrap()),
            value_to_sub,
        );
        assert!(result.is_err())
    }

    #[test]
    pub fn test_input_single_account_verify_non_existing_account() {
        let account = AddressData::account(Discrimination::Test);
        let non_existing_account = AddressData::account(Discrimination::Test);
        let initial_value = Value(100);
        let value_to_sub = Value(110);
        let block0_hash = TestGen::hash();
        let wrong_block0_hash = TestGen::hash();
        let id: Identifier = account.public_key().into();

        let account_ledger = account_ledger_with_initials(&[(id.clone(), initial_value)]);
        let signed_tx = create_empty_transaction(&block0_hash, &account);
        let sign_data_hash = signed_tx.transaction.hash();

        let result = super::input_single_account_verify(
            account_ledger,
            &wrong_block0_hash,
            &sign_data_hash,
            &non_existing_account.public_key().into(),
            &to_account_witness(signed_tx.witnesses.iter().next().unwrap()),
            value_to_sub,
        );
        assert!(result.is_err())
    }

    #[quickcheck]
    pub fn input_utxo_verify_negative_prop_test(
        sign_data_hash: TransactionSignDataHash,
        utxo_pointer: UtxoPointer,
        witness: Witness,
    ) -> TestResult {
        let faucet = AddressData::utxo(Discrimination::Test);
        let message = ledger_mock::create_initial_transaction(Output::from_address(
            faucet.address.clone(),
            Value(100),
        ));
        let (_, ledger) =
            ledger_mock::create_initial_fake_ledger(&[message], ConfigBuilder::new().build())
                .unwrap();
        let result = ledger.apply_input_to_utxo(&sign_data_hash, &utxo_pointer, &witness);
        match (witness, result) {
            (Witness::OldUtxo(_, _), Ok(_)) => {
                TestResult::error("expecting error, but got success")
            }
            (Witness::OldUtxo(_, _), Err(_)) => TestResult::passed(),
            (Witness::Utxo(_), Ok(_)) => TestResult::error("expecting error, but got success"),
            (Witness::Utxo(_), Err(_)) => TestResult::passed(),
            (Witness::Account(_), Ok(_)) => TestResult::error("expecting error, but got success"),
            (Witness::Account(_), Err(_)) => TestResult::passed(),
            (Witness::Multisig(_), _) => TestResult::discard(),
        }
    }

    #[test]
    pub fn test_input_utxo_verify_correct_utxo() {
        let faucet = AddressData::utxo(Discrimination::Test);
        let message = ledger_mock::create_initial_transaction(Output::from_address(
            faucet.address.clone(),
            Value(100),
        ));
        let (block0_hash, ledger) =
            ledger_mock::create_initial_fake_ledger(&[message], ConfigBuilder::new().build())
                .unwrap();
        let utxo = ledger.utxos().next().unwrap();
        let utxo_pointer = UtxoPointer::new(
            utxo.fragment_id,
            utxo.output_index,
            utxo.output.value.clone(),
        );
        let signed_tx = create_empty_transaction(&block0_hash, &faucet);
        let sign_data_hash = signed_tx.transaction.hash();
        let result = ledger.apply_input_to_utxo(
            &sign_data_hash,
            &utxo_pointer,
            &signed_tx.witnesses.iter().next().unwrap(),
        );
        assert!(result.is_ok())
    }

    #[test]
    pub fn test_input_utxo_verify_incorrect_value() {
        let faucet = AddressData::utxo(Discrimination::Test);
        let message = ledger_mock::create_initial_transaction(Output::from_address(
            faucet.address.clone(),
            Value(100),
        ));
        let (block0_hash, ledger) =
            ledger_mock::create_initial_fake_ledger(&[message], ConfigBuilder::new().build())
                .unwrap();
        let utxo = ledger.utxos().next().unwrap();
        let utxo_pointer = UtxoPointer::new(utxo.fragment_id, utxo.output_index, Value(10));
        let signed_tx = create_empty_transaction(&block0_hash, &faucet);
        let sign_data_hash = signed_tx.transaction.hash();
        let result = ledger.apply_input_to_utxo(
            &sign_data_hash,
            &utxo_pointer,
            &signed_tx.witnesses.iter().next().unwrap(),
        );
        assert!(result.is_err())
    }

    #[quickcheck]
    pub fn test_internal_apply_transaction_output_property(
        utxos: utxo::Ledger<Address>,
        accounts: account::Ledger,
        static_params: LedgerStaticParameters,
        transaction_id: FragmentId,
        arbitrary_outputs: ArbitraryAddressDataValueVec,
    ) -> TestResult {
        let multisig_ledger = multisig::Ledger::new();
        let outputs: Vec<Output<Address>> = arbitrary_outputs
            .0
            .iter()
            .map(|x| x.address_data.make_output(x.value.clone()))
            .collect();

        let ledger = build_ledger(utxos, accounts, multisig_ledger, static_params.clone());

        let auth_tx = TransactionBuilder::new()
            .with_outputs(outputs.clone())
            .authenticate()
            .seal();

        let result = ledger.apply_tx_outputs(transaction_id, &auth_tx);

        match (
            should_expect_success(arbitrary_outputs, &static_params),
            result,
        ) {
            (true, Ok(_)) => TestResult::passed(),
            (true, Err(err)) => TestResult::error(format!("Unexpected failure: {:?}", err)),
            (false, Ok(_)) => TestResult::error("Expected failure, but got sucess"),
            (false, Err(_)) => TestResult::passed(),
        }
    }

    fn build_ledger(
        utxos: utxo::Ledger<Address>,
        accounts: account::Ledger,
        multisig_ledger: multisig::Ledger,
        static_params: LedgerStaticParameters,
    ) -> Ledger {
        let mut ledger = Ledger::empty(
            Settings::new(),
            static_params,
            build_time_era(),
            Pots::zero(),
        );

        ledger.utxos = utxos;
        ledger.accounts = accounts;
        ledger.multisig = multisig_ledger;
        ledger
    }

    fn build_time_era() -> TimeEra {
        let now = SystemTime::now();
        let t0 = Timeline::new(now);
        let f0 = SlotDuration::from_secs(5);
        let tf0 = TimeFrame::new(t0, f0);
        let t1 = now + Duration::from_secs(10);
        let slot1 = tf0.slot_at(&t1).unwrap();
        TimeEra::new(slot1, Epoch(2), 4)
    }

    fn should_expect_success(
        arbitrary_outputs: ArbitraryAddressDataValueVec,
        static_params: &LedgerStaticParameters,
    ) -> bool {
        let is_any_address_different_than_ledger_disc = arbitrary_outputs
            .0
            .iter()
            .any(|x| x.address_data.discrimination() != static_params.discrimination);
        let is_any_address_zero_output =
            arbitrary_outputs.0.iter().any(|x| x.value == Value::zero());
        !is_any_address_different_than_ledger_disc && !is_any_address_zero_output
    }

    #[derive(Clone, Debug)]
    pub struct InternalApplyTransactionTestParams {
        pub dyn_params: LedgerParameters,
        pub static_params: LedgerStaticParameters,
        pub transaction_id: Hash,
    }

    impl InternalApplyTransactionTestParams {
        pub fn new() -> Self {
            InternalApplyTransactionTestParams::new_with_fee(LinearFee::new(0, 0, 0))
        }

        pub fn new_with_fee(fees: LinearFee) -> Self {
            let static_params = LedgerStaticParameters {
                block0_initial_hash: TestGen::hash(),
                block0_start_time: config::Block0Date(0),
                discrimination: Discrimination::Test,
                kes_update_speed: 100,
            };

            let dyn_params = LedgerParameters {
                fees: fees,
                reward_params: Some(RewardParams::Linear(0, 0, 0)),
            };
            InternalApplyTransactionTestParams {
                dyn_params: dyn_params,
                static_params: static_params,
                transaction_id: TestGen::hash(),
            }
        }

        pub fn transaction_id(&self) -> Hash {
            self.transaction_id.clone()
        }

        pub fn static_params(&self) -> LedgerStaticParameters {
            self.static_params.clone()
        }

        pub fn expected_account_with_value(&self, value: Value) -> AccountState<()> {
            let account_state = AccountState {
                counter: 0.into(),
                delegation: DelegationType::NonDelegated,
                value: value,
                extra: (),
            };
            account_state
        }

        pub fn expected_utxo_entry<'a>(
            &self,
            output: &'a OutputAddress,
        ) -> utxo::Entry<'a, Address> {
            let utxo = utxo::Entry {
                fragment_id: self.transaction_id(),
                output_index: 0 as u8,
                output: output,
            };
            utxo
        }
    }

    pub struct LedgerStateVerifier {
        multisig_ledger: multisig::Ledger,
        utxos: utxo::Ledger<Address>,
        accounts: account::Ledger,
    }

    impl LedgerStateVerifier {
        pub fn new(ledger: Ledger) -> Self {
            LedgerStateVerifier {
                multisig_ledger: ledger.multisig,
                utxos: ledger.utxos,
                accounts: ledger.accounts,
            }
        }

        pub fn utxo_contains(&self, entry: &utxo::Entry<Address>) -> &Self {
            assert_eq!(
                self.utxos.iter().find(|x| *x == entry.clone()),
                Some(entry.clone())
            );
            self
        }

        pub fn and(&self) -> &Self {
            self
        }

        pub fn accounts_contains(
            &self,
            id: Identifier,
            expected_account_state: AccountState<()>,
        ) -> &Self {
            let account_state = self.accounts.get_state(&id).unwrap();
            assert_eq!(account_state.clone(), expected_account_state);
            self
        }

        pub fn utxos_count_is(&self, count: usize) -> &Self {
            assert_eq!(
                self.utxos.iter().count(),
                count,
                "Utxo count should be equal to {:?}",
                count
            );
            self
        }

        pub fn accounts_count_is(&self, count: usize) -> &Self {
            assert_eq!(
                self.accounts.iter().count(),
                count,
                "Utxo count should be equal to {:?}",
                count
            );
            self
        }

        pub fn multisigs_count_is_zero(&self) -> &Self {
            assert_eq!(self.multisig_ledger.iter_accounts().count(), 0);
            assert_eq!(self.multisig_ledger.iter_declarations().count(), 0);
            self
        }

        pub fn total_value_is(&self, value: Value) -> &Self {
            let account_total = self.accounts.get_total_value().unwrap();
            let multisig_total = self.multisig_ledger.get_total_value().unwrap();
            let utxos_total =
                Value::sum(self.utxos.iter().map(|entry| entry.output.value)).unwrap();
            let totals = vec![account_total, multisig_total, utxos_total];
            let actual_value =
                Value::sum(totals.iter().cloned()).expect("cannot sum up ledger total value");
            assert_eq!(
                value, actual_value,
                "Expected value {:?} vs {:?} of actual",
                value, actual_value
            );
            self
        }

        // Does not cover situation in which we have two identical utxos
        pub fn address_has_expected_balance(&self, address: AddressData, value: Value) -> &Self {
            match self.accounts.exists(&address.to_id()) {
                true => self.account_has_expected_balance(address, value),
                false => self.utxo_has_expected_balance(address, value),
            }
        }

        pub fn account_has_expected_balance(&self, address: AddressData, value: Value) -> &Self {
            let account_state = self
                .accounts
                .get_state(&address.to_id())
                .expect("account does not exists while it should");
            assert_eq!(account_state.value(), value);
            self
        }

        pub fn utxo_has_expected_balance(&self, address_data: AddressData, value: Value) -> &Self {
            let utxo = self
                .utxos
                .iter()
                .find(|x| *x.output == address_data.make_output(value));
            match value == Value::zero() {
                true => {
                    assert!(utxo.is_none());
                    return self;
                }
                false => {
                    let utxo = utxo.unwrap();
                    assert_eq!(utxo.output.value, value);
                    return self;
                }
            }
        }
    }

    #[test]
    pub fn test_internal_apply_transaction_output_delegation_for_existing_account() {
        let params = InternalApplyTransactionTestParams::new();

        let multisig_ledger = multisig::Ledger::new();
        let utxos = utxo::Ledger::new();
        let mut accounts = account::Ledger::new();

        let account = AddressData::account(Discrimination::Test);
        accounts = accounts
            .add_account(&account.to_id(), Value(100), ())
            .unwrap();

        let delegation = AddressData::delegation_for(&account);
        let delegation_output = delegation.make_output(Value(100));

        let ledger = build_ledger(utxos, accounts, multisig_ledger, params.static_params());

        let auth_tx = TransactionBuilder::new()
            .with_output(delegation_output.clone())
            .with_output(account.make_output(Value(1)))
            .authenticate()
            .seal();

        let ledger = ledger
            .apply_tx_outputs(params.transaction_id(), &auth_tx)
            .expect("Unexpected error while applying transaction output");

        LedgerStateVerifier::new(ledger)
            .utxos_count_is(1)
            .and()
            .accounts_count_is(1)
            .and()
            .multisigs_count_is_zero()
            .and()
            .utxo_contains(&params.expected_utxo_entry(&delegation_output))
            .and()
            .accounts_contains(
                account.to_id(),
                params.expected_account_with_value(Value(101)),
            );
    }

    #[test]
    pub fn test_internal_apply_transaction_output_delegation_non_existing_account() {
        let params = InternalApplyTransactionTestParams::new();

        let multisig_ledger = multisig::Ledger::new();
        let utxos = utxo::Ledger::new();
        let accounts = account::Ledger::new();

        let delegation_address = AddressData::delegation(Discrimination::Test);
        let delegation_output = delegation_address.make_output(Value(100));

        let ledger = build_ledger(utxos, accounts, multisig_ledger, params.static_params());

        let auth_tx = TransactionBuilder::new()
            .with_output(delegation_output.clone())
            .authenticate()
            .seal();

        let ledger = ledger
            .apply_tx_outputs(params.transaction_id(), &auth_tx)
            .expect("Unexpected error while applying transaction output");

        LedgerStateVerifier::new(ledger)
            .utxos_count_is(1)
            .and()
            .accounts_count_is(1)
            .and()
            .multisigs_count_is_zero()
            .and()
            .utxo_contains(&params.expected_utxo_entry(&delegation_output))
            .and()
            .accounts_contains(
                delegation_address.delegation_id(),
                params.expected_account_with_value(Value(0)),
            );
    }

    #[test]
    pub fn test_internal_apply_transaction_output_account_for_existing_account() {
        let params = InternalApplyTransactionTestParams::new();

        let multisig_ledger = multisig::Ledger::new();
        let utxos = utxo::Ledger::new();
        let mut accounts = account::Ledger::new();

        let account = AddressData::account(Discrimination::Test);
        accounts = accounts
            .add_account(&account.to_id(), Value(100), ())
            .unwrap();

        let ledger = build_ledger(utxos, accounts, multisig_ledger, params.static_params());

        let auth_tx = TransactionBuilder::new()
            .with_output(account.make_output(Value(200)))
            .authenticate()
            .seal();

        let ledger = ledger
            .apply_tx_outputs(params.transaction_id(), &auth_tx)
            .expect("Unexpected error while applying transaction output");

        LedgerStateVerifier::new(ledger)
            .utxos_count_is(0)
            .and()
            .accounts_count_is(1)
            .and()
            .multisigs_count_is_zero()
            .and()
            .accounts_contains(
                account.to_id(),
                params.expected_account_with_value(Value(300)),
            );
    }

    #[test]
    pub fn test_internal_apply_transaction_output_account_for_non_existing_account() {
        let params = InternalApplyTransactionTestParams::new();

        let multisig_ledger = multisig::Ledger::new();
        let utxos = utxo::Ledger::new();
        let accounts = account::Ledger::new();
        let account = AddressData::account(Discrimination::Test);

        let ledger = build_ledger(utxos, accounts, multisig_ledger, params.static_params());

        let auth_tx = TransactionBuilder::new()
            .with_output(account.make_output(Value(200)))
            .authenticate()
            .seal();

        let ledger = ledger
            .apply_tx_outputs(params.transaction_id(), &auth_tx)
            .expect("Unexpected error while applying transaction output");

        LedgerStateVerifier::new(ledger)
            .utxos_count_is(0)
            .and()
            .accounts_count_is(1)
            .and()
            .multisigs_count_is_zero()
            .and()
            .accounts_contains(
                account.to_id(),
                params.expected_account_with_value(Value(200)),
            );
    }

    #[test]
    pub fn test_internal_apply_transaction_output_empty() {
        let params = InternalApplyTransactionTestParams::new();

        let multisig_ledger = multisig::Ledger::new();
        let utxos = utxo::Ledger::new();
        let accounts = account::Ledger::new();

        let ledger = build_ledger(utxos, accounts, multisig_ledger, params.static_params());

        let auth_tx = TransactionBuilder::new().authenticate().seal();

        let ledger = ledger
            .apply_tx_outputs(params.transaction_id(), &auth_tx)
            .expect("Unexpected error while applying transaction output");

        LedgerStateVerifier::new(ledger)
            .utxos_count_is(0)
            .and()
            .accounts_count_is(0)
            .and()
            .multisigs_count_is_zero();
    }

    /// internal_apply_transaction
    #[test]
    pub fn test_internal_apply_transaction_max_witnesses() {
        let params = InternalApplyTransactionTestParams::new();
        let faucet = AddressDataValue::utxo(Discrimination::Test, Value(100));
        let reciever = AddressData::utxo(Discrimination::Test);
        let (block0_hash, ledger) = ledger_mock::create_fake_ledger_with_faucet(
            &[faucet.clone()],
            ConfigBuilder::new().build(),
        )
        .unwrap();

        let utxo = ledger.utxos().next();

        let auth_tx = TransactionBuilder::new()
            .with_input(faucet.clone().make_input(utxo))
            .with_output(reciever.make_output(Value(100)))
            .authenticate()
            .with_witnesses(
                &block0_hash,
                &iter::from_fn(|| Some(faucet.clone().into()))
                    .take(TX_VERIFY_LIMITS.max_witnesses_count)
                    .collect(),
            )
            .seal();

        assert!(ledger
            .apply_transaction(&params.transaction_id(), &auth_tx, &params.dyn_params)
            .is_err());
    }

    #[test]
    pub fn test_internal_apply_transaction_max_outputs() {
        let params = InternalApplyTransactionTestParams::new();
        let faucet = AddressDataValue::utxo(
            Discrimination::Test,
            Value(TX_VERIFY_LIMITS.max_outputs_count as u64),
        );
        let reciever = AddressData::utxo(Discrimination::Test);
        let (block0_hash, ledger) = ledger_mock::create_fake_ledger_with_faucet(
            &[faucet.clone()],
            ConfigBuilder::new().build(),
        )
        .unwrap();

        let utxo = ledger.utxos().next();

        let auth_tx = TransactionBuilder::new()
            .with_input(faucet.clone().make_input(utxo))
            .with_outputs(
                iter::from_fn(|| Some(reciever.make_output(Value(100))))
                    .take(TX_VERIFY_LIMITS.max_outputs_count)
                    .collect(),
            )
            .authenticate()
            .with_witness(&block0_hash, &faucet.clone().into())
            .seal();

        assert!(ledger
            .apply_transaction(&params.transaction_id(), &auth_tx, &params.dyn_params)
            .is_err());
    }

    #[test]
    pub fn test_internal_apply_transaction_max_inputs() {
        let params = InternalApplyTransactionTestParams::new();
        let faucets: Vec<AddressDataValue> =
            iter::from_fn(|| Some(AddressDataValue::account(Discrimination::Test, Value(1))))
                .take(TX_VERIFY_LIMITS.max_inputs_count + 1)
                .collect();

        let reciever = AddressData::utxo(Discrimination::Test);
        let (block0_hash, ledger) = ledger_mock::create_fake_ledger_with_faucet(
            faucets.as_slice(),
            ConfigBuilder::new().build(),
        )
        .unwrap();

        let auth_tx = TransactionBuilder::new()
            .with_inputs(faucets.iter().map(|x| x.make_input(None)).collect())
            .with_output(
                reciever.make_output(Value((TX_VERIFY_LIMITS.max_inputs_count + 1) as u64)),
            )
            .authenticate()
            .with_witnesses(
                &block0_hash,
                &faucets.iter().cloned().map(|x| x.into()).collect(),
            )
            .seal();

        assert!(ledger
            .apply_transaction(&params.transaction_id(), &auth_tx, &params.dyn_params)
            .is_err());
    }

    #[test]
    pub fn test_internal_apply_transaction_same_witness_for_all_input() {
        let params = InternalApplyTransactionTestParams::new();
        let faucets = &[
            AddressDataValue::account(Discrimination::Test, Value(1)),
            AddressDataValue::account(Discrimination::Test, Value(1)),
        ];
        let reciever = AddressData::utxo(Discrimination::Test);
        let (block0_hash, ledger) =
            ledger_mock::create_fake_ledger_with_faucet(faucets, ConfigBuilder::new().build())
                .unwrap();

        let auth_tx = TransactionBuilder::new()
            .with_inputs(faucets.iter().map(|x| x.make_input(None)).collect())
            .with_output(reciever.make_output(Value(2)))
            .authenticate()
            .with_witnesses(
                &block0_hash,
                &vec![faucets[0].clone().into(), faucets[0].clone().into()],
            )
            .seal();

        assert!(ledger
            .apply_transaction(&params.transaction_id(), &auth_tx, &params.dyn_params)
            .is_err());
    }

    #[test]
    pub fn test_internal_apply_transaction_verify_pot() {
        let mut params = InternalApplyTransactionTestParams::new();
        params.dyn_params = LedgerParameters {
            fees: LinearFee::new(100, 0, 0),
            reward_params: Some(RewardParams::Linear(0, 0, 0)),
        };
        let faucet = AddressDataValue::account(Discrimination::Test, Value(101));
        let reciever = AddressDataValue::account(Discrimination::Test, Value(1));
        let (block0_hash, ledger) = ledger_mock::create_fake_ledger_with_faucet(
            &[faucet.clone()],
            ConfigBuilder::new().build(),
        )
        .unwrap();

        let auth_tx = TransactionBuilder::new()
            .with_input(faucet.make_input(None))
            .with_output(reciever.make_output())
            .authenticate()
            .with_witness(&block0_hash, &faucet.into())
            .seal();

        let result =
            ledger.apply_transaction(&params.transaction_id(), &auth_tx, &params.dyn_params);
        assert!(result.is_ok(), "{:?}", result.err());
        let (ledger, _) = result.unwrap();

        assert_eq!(ledger.pots.fees, Value(100));
    }

    #[test]
    pub fn test_internal_apply_transaction_witnesses_count_are_grater_than_inputs() {
        let params = InternalApplyTransactionTestParams::new();
        let faucets = &[
            AddressDataValue::account(Discrimination::Test, Value(1)),
            AddressDataValue::account(Discrimination::Test, Value(1)),
        ];
        let reciever = AddressData::utxo(Discrimination::Test);
        let (block0_hash, ledger) =
            ledger_mock::create_fake_ledger_with_faucet(faucets, ConfigBuilder::new().build())
                .unwrap();

        let auth_tx = TransactionBuilder::new()
            .with_input(faucets[0].make_input(None))
            .with_output(reciever.make_output(Value(2)))
            .authenticate()
            .with_witnesses(
                &block0_hash,
                &vec![faucets[0].clone().into(), faucets[1].clone().into()],
            )
            .seal();

        assert!(ledger
            .apply_transaction(&params.transaction_id(), &auth_tx, &params.dyn_params)
            .is_err());
    }

    #[test]
    pub fn test_internal_apply_transaction_witnesses_count_are_smaller_than_inputs() {
        let params = InternalApplyTransactionTestParams::new();
        let faucets = &[
            AddressDataValue::account(Discrimination::Test, Value(1)),
            AddressDataValue::account(Discrimination::Test, Value(1)),
        ];
        let reciever = AddressData::utxo(Discrimination::Test);
        let (block0_hash, ledger) =
            ledger_mock::create_fake_ledger_with_faucet(faucets, ConfigBuilder::new().build())
                .unwrap();

        let auth_tx = TransactionBuilder::new()
            .with_inputs(vec![
                faucets[0].make_input(None),
                faucets[1].make_input(None),
            ])
            .with_output(reciever.make_output(Value(2)))
            .authenticate()
            .with_witness(&block0_hash, &faucets[0].clone().into())
            .seal();

        assert!(ledger
            .apply_transaction(&params.transaction_id(), &auth_tx, &params.dyn_params)
            .is_err());
    }

    #[quickcheck]
    pub fn test_internal_apply_transaction_is_balanced(
        input_addresses: ArbitraryAddressDataValueVec,
        output_addresses: ArbitraryAddressDataValueVec,
        fee: Value,
    ) -> TestResult {
        if input_addresses.is_empty() || output_addresses.is_empty() {
            return TestResult::discard();
        }

        let params = InternalApplyTransactionTestParams::new_with_fee(LinearFee::new(fee.0, 0, 0));
        let (block0_hash, ledger) = ledger_mock::create_fake_ledger_with_faucet(
            &input_addresses.values().as_slice(),
            ConfigBuilder::new().build(),
        )
        .unwrap();

        let auth_tx = TransactionBuilder::new()
            .with_inputs(input_addresses.make_inputs(ledger.utxos.clone()))
            .with_outputs(output_addresses.make_outputs())
            .authenticate()
            .with_witnesses(&block0_hash, &input_addresses.as_addresses())
            .seal();

        let balance_res = (input_addresses.total_value() - output_addresses.total_value())
            .and_then(|balance| balance - fee);
        match (
            balance_res,
            ledger.apply_transaction(&params.transaction_id(), &auth_tx, &params.dyn_params),
        ) {
            (Ok(balance), Ok(_)) => TestResult::from_bool(balance == Value::zero()),
            (Err(err), Ok(_)) => TestResult::error(format!(
                "Expected balance is non zero {:?}, yet transaction is accepted",
                err
            )),
            (Ok(balance), Err(_)) => TestResult::from_bool(balance != Value::zero()),
            (Err(_), Err(_)) => TestResult::passed(),
        }
    }

    #[test]
    pub fn test_internal_apply_transaction_witness_collection_should_be_ordered_as_inputs() {
        let params = InternalApplyTransactionTestParams::new();
        let faucets = &[
            AddressDataValue::account(Discrimination::Test, Value(1)),
            AddressDataValue::account(Discrimination::Test, Value(1)),
        ];
        let reciever = AddressData::utxo(Discrimination::Test);
        let (block0_hash, ledger) =
            ledger_mock::create_fake_ledger_with_faucet(faucets, ConfigBuilder::new().build())
                .unwrap();

        let auth_tx = TransactionBuilder::new()
            .with_inputs(vec![
                faucets[0].make_input(None),
                faucets[1].make_input(None),
            ])
            .with_output(reciever.make_output(Value(2)))
            .authenticate()
            .with_witnesses(
                &block0_hash,
                &vec![faucets[1].clone().into(), faucets[0].clone().into()],
            )
            .seal();

        assert!(ledger
            .apply_transaction(&params.transaction_id(), &auth_tx, &params.dyn_params)
            .is_err());
    }

    #[test]
    pub fn test_internal_apply_transaction_no_inputs_outputs() {
        let params = InternalApplyTransactionTestParams::new();
        let faucet = AddressDataValue::account(Discrimination::Test, Value(1));
        let (block0_hash, ledger) = ledger_mock::create_fake_ledger_with_faucet(
            &[faucet.clone()],
            ConfigBuilder::new().build(),
        )
        .unwrap();

        let auth_tx = create_empty_transaction(&block0_hash, &faucet.into());

        assert!(ledger
            .apply_transaction(&params.transaction_id(), &auth_tx, &params.dyn_params)
            .is_err());
    }

    #[quickcheck]
    pub fn test_internal_apply_transaction_funds_were_transfered(
        sender_address: AddressData,
        reciever_address: AddressData,
    ) {
        let params = InternalApplyTransactionTestParams::new();
        let faucet = AddressDataValue::new(sender_address, Value(1));
        let reciever = AddressDataValue::new(reciever_address, Value(1));
        let (block0_hash, ledger) = ledger_mock::create_fake_ledger_with_faucet(
            &[faucet.clone()],
            ConfigBuilder::new().build(),
        )
        .unwrap();

        let utxo = ledger.utxos().next();

        let auth_tx = TransactionBuilder::new()
            .with_input(faucet.make_input(utxo))
            .with_output(reciever.make_output())
            .authenticate()
            .with_witness(&block0_hash, &faucet.clone().into())
            .seal();

        let result =
            ledger.apply_transaction(&params.transaction_id(), &auth_tx, &params.dyn_params);
        assert!(result.is_ok());
        let (ledger, _) = result.unwrap();

        LedgerStateVerifier::new(ledger)
            .address_has_expected_balance(reciever.into(), Value(1))
            .and()
            .address_has_expected_balance(faucet.into(), Value(0))
            .and()
            .total_value_is(Value(1));
    }

    #[test]
    pub fn test_internal_apply_transaction_wrong_witness_type() {
        let params = InternalApplyTransactionTestParams::new();
        let faucet = AddressDataValue::utxo(Discrimination::Test, Value(1));
        let reciever = AddressDataValue::account(Discrimination::Test, Value(1));
        let (block0_hash, ledger) = ledger_mock::create_fake_ledger_with_faucet(
            &[faucet.clone()],
            ConfigBuilder::new().build(),
        )
        .unwrap();

        let utxo = ledger.utxos().next();

        let mut finalizer = TransactionBuilder::new()
            .with_input(faucet.make_input(utxo))
            .with_output(reciever.make_output())
            .authenticate();

        let witness = witness_builder::make_account_witness(
            &block0_hash,
            &SpendingCounter::zero(),
            &faucet.private_key(),
            &finalizer.transaction_hash(),
        );

        let auth_tx = finalizer.with_witness_from(witness).seal();

        assert!(ledger
            .apply_transaction(&params.transaction_id(), &auth_tx, &params.dyn_params)
            .is_err());
    }

    #[test]
    pub fn test_internal_apply_transaction_wrong_transaction_hash() {
        let params = InternalApplyTransactionTestParams::new();
        let faucet = AddressDataValue::utxo(Discrimination::Test, Value(1));
        let reciever = AddressDataValue::account(Discrimination::Test, Value(1));
        let (block0_hash, ledger) = ledger_mock::create_fake_ledger_with_faucet(
            &[faucet.clone()],
            ConfigBuilder::new().build(),
        )
        .unwrap();

        let fake_transaction = create_empty_transaction(&block0_hash, &faucet.clone().into());
        let fake_transaction_hash = fake_transaction.transaction.hash();
        let utxo = ledger.utxos().next();

        let mut finalizer = TransactionBuilder::new()
            .with_input(faucet.make_input(utxo))
            .with_output(reciever.make_output())
            .authenticate();

        let witness = witness_builder::make_account_witness(
            &block0_hash,
            &SpendingCounter::zero(),
            &faucet.private_key(),
            &fake_transaction_hash,
        );

        let auth_tx = finalizer.with_witness_from(witness).seal();

        assert!(ledger
            .apply_transaction(&params.transaction_id(), &auth_tx, &params.dyn_params)
            .is_err());
    }

    #[test]
    pub fn test_internal_apply_transaction_wrong_block0_hash() {
        let params = InternalApplyTransactionTestParams::new();
        let wrong_block0_hash = TestGen::hash();
        let faucet = AddressDataValue::account(Discrimination::Test, Value(1));
        let reciever = AddressDataValue::account(Discrimination::Test, Value(1));
        let (_, ledger) = ledger_mock::create_fake_ledger_with_faucet(
            &[faucet.clone()],
            ConfigBuilder::new().build(),
        )
        .unwrap();

        let mut finalizer = TransactionBuilder::new()
            .with_input(faucet.make_input(None))
            .with_output(reciever.make_output())
            .authenticate();
        let witness = witness_builder::make_account_witness(
            &wrong_block0_hash,
            &SpendingCounter::zero(),
            &faucet.private_key(),
            &finalizer.transaction_hash(),
        );
        let auth_tx = finalizer.with_witness_from(witness).seal();
        assert!(ledger
            .apply_transaction(&params.transaction_id(), &auth_tx, &params.dyn_params)
            .is_err());
    }

    #[test]
    pub fn test_internal_apply_transaction_wrong_spending_counter() {
        let params = InternalApplyTransactionTestParams::new();
        let faucet = AddressDataValue::utxo(Discrimination::Test, Value(1));
        let reciever = AddressDataValue::account(Discrimination::Test, Value(1));
        let (block0_hash, ledger) = ledger_mock::create_fake_ledger_with_faucet(
            &[faucet.clone()],
            ConfigBuilder::new().build(),
        )
        .unwrap();
        let utxo = ledger.utxos().next();

        let mut finalizer = TransactionBuilder::new()
            .with_input(faucet.make_input(utxo))
            .with_output(reciever.make_output())
            .authenticate();

        let witness = witness_builder::make_account_witness(
            &block0_hash,
            &1.into(),
            &faucet.private_key(),
            &finalizer.transaction_hash(),
        );
        let auth_tx = finalizer.with_witness_from(witness).seal();
        assert!(ledger
            .apply_transaction(&params.transaction_id(), &auth_tx, &params.dyn_params)
            .is_err());
    }

    #[test]
    pub fn test_internal_apply_transaction_wrong_private_key() {
        let params = InternalApplyTransactionTestParams::new();
        let faucet = AddressDataValue::utxo(Discrimination::Test, Value(1));
        let reciever = AddressDataValue::account(Discrimination::Test, Value(1));
        let (block0_hash, ledger) = ledger_mock::create_fake_ledger_with_faucet(
            &[faucet.clone()],
            ConfigBuilder::new().build(),
        )
        .unwrap();
        let utxo = ledger.utxos().next();

        let mut finalizer = TransactionBuilder::new()
            .with_input(faucet.make_input(utxo))
            .with_output(reciever.make_output())
            .authenticate();

        let witness = witness_builder::make_account_witness(
            &block0_hash,
            &SpendingCounter::zero(),
            &reciever.private_key(),
            &finalizer.transaction_hash(),
        );
        let auth_tx = finalizer.with_witness_from(witness).seal();
        assert!(ledger
            .apply_transaction(&params.transaction_id(), &auth_tx, &params.dyn_params)
            .is_err());
    }
    */
}
