//! Mockchain ledger. Ledger exists in order to update the
//! current state and verify transactions.

use super::check;
use crate::block::{
    BlockDate, ChainLength, ConsensusVersion, HeaderContentEvalContext, HeaderHash,
};
use crate::config::{self, ConfigParam};
use crate::fee::{FeeAlgorithm, LinearFee};
use crate::fragment::{Fragment, FragmentId};
use crate::leadership::genesis::ActiveSlotsCoeffError;
use crate::stake::{DelegationError, DelegationState, StakeDistribution};
use crate::transaction::*;
use crate::value::*;
use crate::{account, certificate, legacy, multisig, setting, stake, update, utxo};
use chain_addr::{Address, Discrimination, Kind};
use chain_core::property::{self, ChainLength as _, Fragment as _};
use chain_time::{Epoch, SlotDuration, TimeEra, TimeFrame, Timeline};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

// static parameters, effectively this is constant in the parameter of the blockchain
#[derive(Clone, PartialEq, Eq)]
pub struct LedgerStaticParameters {
    pub block0_initial_hash: HeaderHash,
    pub block0_start_time: config::Block0Date,
    pub discrimination: Discrimination,
    pub kes_update_speed: u32,
}

// parameters to validate ledger
#[derive(Clone)]
pub struct LedgerParameters {
    pub fees: LinearFee,
}

//Limits for input/output transactions and witnesses
const MAX_TRANSACTION_INPUTS_COUNT: usize = 256;
const MAX_TRANSACTION_OUTPUTS_COUNT: usize = 254;
const MAX_TRANSACTION_WITNESSES_COUNT: usize = 256;

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
    pub(crate) delegation: DelegationState,
    pub(crate) static_params: Arc<LedgerStaticParameters>,
    pub(crate) date: BlockDate,
    pub(crate) chain_length: ChainLength,
    pub(crate) era: TimeEra,
    pub(crate) pot: Value,
}

custom_error! {
    #[derive(Clone, PartialEq, Eq)]
    pub Block0Error
        OnlyMessageReceived = "Old UTxOs and Initial Message are not valid in a normal block",
        TransactionHasInput = "Transaction should not have inputs in a block0",
        TransactionHasOutput = "Transaction should not have outputs in a block0",
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
}

pub type OutputOldAddress = Output<legacy::OldAddress>;
pub type OutputAddress = Output<Address>;

custom_error! {
    #[derive(Clone, PartialEq, Eq)]
    pub Error
        Config { source: config::Error } = "Invalid settings",
        NotEnoughSignatures { actual: usize, expected: usize } = "Not enough signatures, expected {expected} signatures but received {actual}",
        UtxoValueNotMatching { expected: Value, value: Value } = "The UTxO value ({expected}) in the transaction does not match the actually state value: {value}",
        UtxoError { source: utxo::Error } = "Invalid UTxO",
        UtxoInvalidSignature { utxo: UtxoPointer, output: OutputAddress, witness: Witness } = "Transaction with invalid signature",
        OldUtxoInvalidSignature { utxo: UtxoPointer, output: OutputOldAddress, witness: Witness } = "Old Transaction with invalid signature",
        OldUtxoInvalidPublicKey { utxo: UtxoPointer, output: OutputOldAddress, witness: Witness } = "Old Transaction with invalid public key",
        AccountInvalidSignature { account: account::Identifier, witness: Witness } = "Account with invalid signature",
        MultisigInvalidSignature { multisig: multisig::Identifier, witness: Witness } = "Multisig with invalid signature",
        TransactionHasTooManyInputs {expected: usize, actual: usize } = "Transaction has more than {expected} inputs ({actual})",
        TransactionHasTooManyOutputs {expected: usize, actual: usize } = "Transaction has more than {expected} outputs ({actual})",
        TransactionHasTooManyWitnesses {expected: usize, actual: usize } = "Transaction has more than {expected} witnesses ({actual})",
        FeeCalculationError { error: ValueError } = "Error while computing the fees: {error}",
        PraosActiveSlotsCoeffInvalid { error: ActiveSlotsCoeffError } = "Praos active slot coefficient invalid: {error}",
        UtxoInputsTotal { error: ValueError } = "Error while computing the transaction's total input: {error}",
        UtxoOutputsTotal { error: ValueError } = "Error while computing the transaction's total output: {error}",
        Block0 { source: Block0Error } = "Invalid Block0",
        Account { source: account::LedgerError } = "Error or Invalid account",
        Multisig { source: multisig::LedgerError } = "Error or Invalid multisig",
        NotBalanced { inputs: Value, outputs: Value } = "Inputs, outputs and fees are not balanced, transaction with {inputs} input and {outputs} output",
        ZeroOutput { output: Output<Address> } = "Empty output",
        OutputGroupInvalid { output: Output<Address> } = "Output group invalid",
        Delegation { source: DelegationError } = "Error or Invalid delegation ",
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
}

impl Ledger {
    fn empty(
        settings: setting::Settings,
        static_params: LedgerStaticParameters,
        era: TimeEra,
    ) -> Self {
        Ledger {
            utxos: utxo::Ledger::new(),
            oldutxos: utxo::Ledger::new(),
            accounts: account::Ledger::new(),
            settings,
            updates: update::UpdateState::new(),
            multisig: multisig::Ledger::new(),
            delegation: DelegationState::new(),
            static_params: Arc::new(static_params),
            date: BlockDate::first(),
            chain_length: ChainLength(0),
            era,
            pot: Value::zero(),
        }
    }

    pub fn new<'a, I>(block0_initial_hash: HeaderHash, contents: I) -> Result<Self, Error>
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

        let mut regular_ents = crate::fragment::ConfigParams::new();
        let mut block0_start_time = None;
        let mut slot_duration = None;
        let mut discrimination = None;
        let mut slots_per_epoch = None;
        let mut kes_update_speed = None;

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

        let mut ledger = Ledger::empty(settings, static_params, era);

        let ledger_params = ledger.get_ledger_parameters();

        for content in content_iter {
            let fragment_id = content.id();
            match content {
                Fragment::Initial(_) => {
                    return Err(Error::Block0 {
                        source: Block0Error::InitialMessageMany,
                    });
                }
                Fragment::OldUtxoDeclaration(old) => {
                    ledger.oldutxos = apply_old_declaration(&fragment_id, ledger.oldutxos, old)?;
                }
                Fragment::Transaction(authenticated_tx) => {
                    if authenticated_tx.transaction.inputs.len() != 0 {
                        return Err(Error::Block0 {
                            source: Block0Error::TransactionHasInput,
                        });
                    }
                    if authenticated_tx.witnesses.len() != 0 {
                        return Err(Error::Block0 {
                            source: Block0Error::TransactionHasWitnesses,
                        });
                    }
                    let (new_utxos, new_accounts, new_multisig) =
                        internal_apply_transaction_output(
                            ledger.utxos,
                            ledger.accounts,
                            ledger.multisig,
                            &ledger.static_params,
                            &ledger_params,
                            &fragment_id,
                            &authenticated_tx.transaction.outputs,
                        )?;
                    ledger.utxos = new_utxos;
                    ledger.accounts = new_accounts;
                    ledger.multisig = new_multisig;
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
                Fragment::Certificate(authenticated_cert_tx) => {
                    if authenticated_cert_tx.transaction.inputs.len() != 0 {
                        return Err(Error::Block0 {
                            source: Block0Error::TransactionHasInput,
                        });
                    }
                    if authenticated_cert_tx.witnesses.len() != 0 {
                        return Err(Error::Block0 {
                            source: Block0Error::TransactionHasWitnesses,
                        });
                    }
                    if authenticated_cert_tx.transaction.outputs.len() != 0 {
                        return Err(Error::Block0 {
                            source: Block0Error::TransactionHasOutput,
                        });
                    }
                    ledger = ledger
                        .apply_certificate_content(&authenticated_cert_tx.transaction.extra)?;
                }
            }
        }

        ledger.validate_utxo_total_value()?;
        Ok(ledger)
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

        new_ledger.chain_length = self.chain_length.next();

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

        let (updates, settings) = new_ledger.updates.process_proposals(
            new_ledger.settings,
            new_ledger.date,
            metadata.block_date,
        )?;
        new_ledger.updates = updates;
        new_ledger.settings = settings;

        for content in contents {
            new_ledger = new_ledger.apply_fragment(ledger_params, content, metadata)?;
        }

        new_ledger.date = metadata.block_date;
        metadata
            .nonce
            .as_ref()
            .map(|n| new_ledger.settings.consensus_nonce.hash_with(n));
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
        metadata: &HeaderContentEvalContext,
    ) -> Result<Self, Error> {
        let mut new_ledger = self.clone();

        let fragment_id = content.id();
        match content {
            Fragment::Initial(_) => {
                return Err(Error::Block0 {
                    source: Block0Error::OnlyMessageReceived,
                })
            }
            Fragment::OldUtxoDeclaration(_) => {
                return Err(Error::Block0 {
                    source: Block0Error::OnlyMessageReceived,
                });
            }
            Fragment::Transaction(authenticated_tx) => {
                let (new_ledger_, _fee) = new_ledger.apply_transaction(
                    &fragment_id,
                    &authenticated_tx,
                    &ledger_params,
                )?;
                new_ledger = new_ledger_;
            }
            Fragment::OwnerStakeDelegation(osd_tx) => {
                let (new_ledger_, _fee) =
                    new_ledger.apply_owner_stake_delegation(&osd_tx, &ledger_params)?;
                new_ledger = new_ledger_;
            }
            Fragment::UpdateProposal(update_proposal) => {
                new_ledger = new_ledger.apply_update_proposal(
                    content.id(),
                    &update_proposal,
                    metadata.block_date,
                )?;
            }
            Fragment::UpdateVote(vote) => {
                new_ledger = new_ledger.apply_update_vote(&vote)?;
            }
            Fragment::Certificate(authenticated_cert_tx) => {
                let (new_ledger_, _fee) = new_ledger.apply_certificate(
                    &fragment_id,
                    authenticated_cert_tx,
                    &ledger_params,
                )?;
                new_ledger = new_ledger_;
            }
        }

        Ok(new_ledger)
    }

    pub fn apply_transaction<Extra>(
        mut self,
        fragment_id: &FragmentId,
        signed_tx: &AuthenticatedTransaction<Address, Extra>,
        dyn_params: &LedgerParameters,
    ) -> Result<(Self, Value), Error>
    where
        Extra: property::Serialize,
        LinearFee: FeeAlgorithm<Transaction<Address, Extra>>,
    {
        let sign_data_hash = signed_tx.transaction.hash();
        let fee = dyn_params
            .fees
            .calculate(&signed_tx.transaction)
            .map(Ok)
            .unwrap_or(Err(Error::FeeCalculationError {
                error: ValueError::Overflow,
            }))?;
        self = internal_apply_transaction(
            self,
            dyn_params,
            &fragment_id,
            &sign_data_hash,
            &signed_tx.transaction.inputs[..],
            &signed_tx.transaction.outputs[..],
            &signed_tx.witnesses[..],
            fee,
        )?;
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

    fn apply_certificate_content(
        mut self,
        certificate: &certificate::Certificate,
    ) -> Result<Self, Error> {
        match certificate.content {
            certificate::CertificateContent::StakeDelegation(ref reg) => {
                if !self.delegation.stake_pool_exists(&reg.pool_id) {
                    return Err(DelegationError::StakeDelegationPoolKeyIsInvalid(
                        reg.pool_id.clone(),
                    )
                    .into());
                }

                if let Some(account_key) = reg.stake_key_id.to_single_account() {
                    self.accounts = self
                        .accounts
                        .set_delegation(&account_key, Some(reg.pool_id.clone()))?;
                } else {
                    return Err(DelegationError::StakeDelegationAccountIsInvalid(
                        reg.stake_key_id.clone(),
                    )
                    .into());
                }
            }
            certificate::CertificateContent::StakePoolRegistration(ref reg) => {
                self.delegation = self.delegation.register_stake_pool(reg.clone())?
            }
            certificate::CertificateContent::StakePoolRetirement(ref reg) => {
                self.delegation = self.delegation.deregister_stake_pool(&reg.pool_id)?
            }
        }
        Ok(self)
    }

    pub fn apply_certificate(
        mut self,
        fragment_id: &FragmentId,
        auth_cert: &AuthenticatedTransaction<Address, certificate::Certificate>,
        dyn_params: &LedgerParameters,
    ) -> Result<(Self, Value), Error> {
        let verified = auth_cert.transaction.extra.verify();
        if verified == chain_crypto::Verification::Failed {
            return Err(Error::CertificateInvalidSignature);
        };
        let (new_ledger, fee) = self.apply_transaction(fragment_id, auth_cert, dyn_params)?;

        self = new_ledger.apply_certificate_content(&auth_cert.transaction.extra)?;

        Ok((self, fee))
    }

    pub fn apply_owner_stake_delegation(
        mut self,
        auth_cert: &AuthenticatedTransaction<Address, certificate::OwnerStakeDelegation>,
        dyn_params: &LedgerParameters,
    ) -> Result<(Self, Value), Error> {
        let sign_data_hash = auth_cert.transaction.hash();

        let (account_id, value, witness) = {
            check::valid_stake_owner_delegation_transaction(&auth_cert)?;

            let input = &auth_cert.transaction.inputs[0];
            match input.to_enum() {
                InputEnum::UtxoInput(_) => {
                    return Err(Error::OwnerStakeDelegationInvalidTransaction);
                }
                InputEnum::AccountInput(account_id, value) => {
                    (account_id, value, &auth_cert.witnesses[0])
                }
            }
        };

        let fee = dyn_params
            .fees
            .calculate(&auth_cert.transaction)
            .map(Ok)
            .unwrap_or(Err(Error::FeeCalculationError {
                error: ValueError::Overflow,
            }))?;
        if fee != value {
            return Err(Error::NotBalanced {
                inputs: value,
                outputs: fee,
            });
        }

        match match_identifier_witness(&account_id, witness)? {
            MatchingIdentifierWitness::Single(account_id, witness) => {
                let single = input_single_account_verify(
                    self.accounts,
                    &self.static_params.block0_initial_hash,
                    &sign_data_hash,
                    &account_id,
                    witness,
                    value,
                )?;
                self.accounts = single.set_delegation(&account_id, Some(auth_cert.transaction.extra.stake_pool.clone()))?;
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
                self.multisig = multi.set_delegation(&account_id, Some(auth_cert.transaction.extra.stake_pool.clone()))?;
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
        }
    }

    pub fn consensus_version(&self) -> ConsensusVersion {
        self.settings.consensus_version
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

    pub fn delegation(&self) -> &DelegationState {
        &self.delegation
    }

    pub fn delegation_mut(&mut self) -> &mut DelegationState {
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
            .chain(Some(self.pot));
        Value::sum(all_utxo_values).map_err(|_| Error::Block0 {
            source: Block0Error::UtxoTotalValueTooBig,
        })?;
        Ok(())
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

/// Apply the transaction
fn internal_apply_transaction(
    mut ledger: Ledger,
    dyn_params: &LedgerParameters,
    fragment_id: &FragmentId,
    sign_data_hash: &TransactionSignDataHash,
    inputs: &[Input],
    outputs: &[Output<Address>],
    witnesses: &[Witness],
    fee: Value,
) -> Result<Ledger, Error> {
    if inputs.len() > MAX_TRANSACTION_INPUTS_COUNT {
        return Err(Error::TransactionHasTooManyInputs {
            expected: MAX_TRANSACTION_INPUTS_COUNT,
            actual: inputs.len(),
        });
    }

    if outputs.len() > MAX_TRANSACTION_OUTPUTS_COUNT {
        return Err(Error::TransactionHasTooManyOutputs {
            expected: MAX_TRANSACTION_OUTPUTS_COUNT,
            actual: outputs.len(),
        });
    }

    if witnesses.len() > MAX_TRANSACTION_WITNESSES_COUNT {
        return Err(Error::TransactionHasTooManyWitnesses {
            expected: MAX_TRANSACTION_WITNESSES_COUNT,
            actual: witnesses.len(),
        });
    }

    // 1. verify that number of signatures matches number of
    // transactions
    if inputs.len() != witnesses.len() {
        return Err(Error::NotEnoughSignatures {
            expected: inputs.len(),
            actual: witnesses.len(),
        });
    }

    // 2. validate inputs of transaction by gathering what we know of it,
    // then verifying the associated witness
    for (input, witness) in inputs.iter().zip(witnesses.iter()) {
        match input.to_enum() {
            InputEnum::UtxoInput(utxo) => {
                ledger = input_utxo_verify(ledger, sign_data_hash, &utxo, witness)?
            }
            InputEnum::AccountInput(account_id, value) => {
                match match_identifier_witness(&account_id, witness)? {
                    MatchingIdentifierWitness::Single(account_id, witness) => {
                        let single = input_single_account_verify(
                            ledger.accounts,
                            &ledger.static_params.block0_initial_hash,
                            sign_data_hash,
                            &account_id,
                            witness,
                            value,
                        )?;
                        ledger.accounts = single;
                    }
                    MatchingIdentifierWitness::Multi(account_id, witness) => {
                        let multi = input_multi_account_verify(
                            ledger.multisig,
                            &ledger.static_params.block0_initial_hash,
                            sign_data_hash,
                            &account_id,
                            witness,
                            value,
                        )?;
                        ledger.multisig = multi;
                    }
                }
            }
        }
    }

    // 3. verify that transaction sum is zero.
    let total_input = Value::sum(inputs.iter().map(|i| i.value))
        .map_err(|e| Error::UtxoInputsTotal { error: e })?;
    let total_output = Value::sum(outputs.iter().map(|i| i.value).chain(std::iter::once(fee)))
        .map_err(|e| Error::UtxoOutputsTotal { error: e })?;
    if total_input != total_output {
        return Err(Error::NotBalanced {
            inputs: total_input,
            outputs: total_output,
        });
    }

    // 4. add the new outputs
    let (new_utxos, new_accounts, new_multisig) = internal_apply_transaction_output(
        ledger.utxos,
        ledger.accounts,
        ledger.multisig,
        &ledger.static_params,
        dyn_params,
        fragment_id,
        outputs,
    )?;
    ledger.utxos = new_utxos;
    ledger.accounts = new_accounts;
    ledger.multisig = new_multisig;

    // 5. add fee to pot
    ledger.pot = (ledger.pot + fee).map_err(|error| Error::PotValueInvalid { error })?;

    Ok(ledger)
}

fn internal_apply_transaction_output(
    mut utxos: utxo::Ledger<Address>,
    mut accounts: account::Ledger,
    mut multisig: multisig::Ledger,
    static_params: &LedgerStaticParameters,
    _dyn_params: &LedgerParameters,
    transaction_id: &FragmentId,
    outputs: &[Output<Address>],
) -> Result<(utxo::Ledger<Address>, account::Ledger, multisig::Ledger), Error> {
    let mut new_utxos = Vec::new();
    for (index, output) in outputs.iter().enumerate() {
        // Reject zero-valued outputs.
        if output.value == Value::zero() {
            return Err(Error::ZeroOutput {
                output: output.clone(),
            });
        }

        if output.address.discrimination() != static_params.discrimination {
            return Err(Error::InvalidDiscrimination);
        }
        match output.address.kind() {
            Kind::Single(_) => {
                new_utxos.push((index as u8, output.clone()));
            }
            Kind::Group(_, account_id) => {
                let account_id = account_id.clone().into();
                // TODO: probably faster to just call add_account and check for already exists error
                if !accounts.exists(&account_id) {
                    accounts = accounts.add_account(&account_id, Value::zero(), ())?;
                }
                new_utxos.push((index as u8, output.clone()));
            }
            Kind::Account(identifier) => {
                // don't have a way to make a newtype ref from the ref so .clone()
                let account = identifier.clone().into();
                accounts = match accounts.add_value(&account, output.value) {
                    Ok(accounts) => accounts,
                    Err(account::LedgerError::NonExistent) => {
                        accounts.add_account(&account, output.value, ())?
                    }
                    Err(error) => return Err(error.into()),
                };
            }
            Kind::Multisig(identifier) => {
                let identifier = multisig::Identifier::from(identifier.clone());
                multisig = multisig.add_value(&identifier, output.value)?;
            }
        }
    }

    utxos = utxos.add(transaction_id, &new_utxos)?;
    Ok((utxos, accounts, multisig))
}

fn input_utxo_verify(
    mut ledger: Ledger,
    sign_data_hash: &TransactionSignDataHash,
    utxo: &UtxoPointer,
    witness: &Witness,
) -> Result<Ledger, Error> {
    match witness {
        Witness::Account(_) => Err(Error::ExpectingUtxoWitness),
        Witness::Multisig(_) => Err(Error::ExpectingUtxoWitness),
        Witness::OldUtxo(xpub, signature) => {
            let (old_utxos, associated_output) = ledger
                .oldutxos
                .remove(&utxo.transaction_id, utxo.output_index)?;

            ledger.oldutxos = old_utxos;
            if utxo.value != associated_output.value {
                return Err(Error::UtxoValueNotMatching {
                    expected: utxo.value,
                    value: associated_output.value,
                });
            };

            if legacy::oldaddress_from_xpub(&associated_output.address, xpub) {
                return Err(Error::OldUtxoInvalidPublicKey {
                    utxo: utxo.clone(),
                    output: associated_output.clone(),
                    witness: witness.clone(),
                });
            };

            let data_to_verify =
                WitnessUtxoData::new(&ledger.static_params.block0_initial_hash, sign_data_hash);
            let verified = signature.verify(&xpub, &data_to_verify);
            if verified == chain_crypto::Verification::Failed {
                return Err(Error::OldUtxoInvalidSignature {
                    utxo: utxo.clone(),
                    output: associated_output.clone(),
                    witness: witness.clone(),
                });
            };

            Ok(ledger)
        }
        Witness::Utxo(signature) => {
            let (new_utxos, associated_output) = ledger
                .utxos
                .remove(&utxo.transaction_id, utxo.output_index)?;
            ledger.utxos = new_utxos;
            if utxo.value != associated_output.value {
                return Err(Error::UtxoValueNotMatching {
                    expected: utxo.value,
                    value: associated_output.value,
                });
            }

            let data_to_verify =
                WitnessUtxoData::new(&ledger.static_params.block0_initial_hash, sign_data_hash);
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
            Ok(ledger)
        }
    }
}

pub enum MatchingIdentifierWitness<'a> {
    Single(account::Identifier, &'a account::Witness),
    Multi(multisig::Identifier, &'a multisig::Witness),
}

fn match_identifier_witness<'a>(
    account: &AccountIdentifier,
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
    block0_hash: &HeaderHash,
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
    block0_hash: &HeaderHash,
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

pub enum Entry<'a> {
    Globals(Globals),
    Utxo(utxo::Entry<'a, Address>),
    OldUtxo(utxo::Entry<'a, legacy::OldAddress>),
    Account(
        (
            &'a account::Identifier,
            &'a crate::accounting::account::AccountState<()>,
        ),
    ),
    ConfigParam(ConfigParam),
    UpdateProposal(
        (
            &'a crate::update::UpdateProposalId,
            &'a crate::update::UpdateProposalState,
        ),
    ),
    MultisigAccount(
        (
            &'a crate::multisig::Identifier,
            &'a crate::accounting::account::AccountState<()>,
        ),
    ),
    MultisigDeclaration(
        (
            &'a crate::multisig::Identifier,
            &'a crate::multisig::Declaration,
        ),
    ),
    StakePool(
        (
            &'a crate::stake::StakePoolId,
            &'a crate::stake::StakePoolInfo,
        ),
    ),
}

pub struct Globals {
    pub date: BlockDate,
    pub chain_length: ChainLength,
    pub static_params: LedgerStaticParameters,
    pub era: TimeEra,
}

enum IterState<'a> {
    Initial,
    Utxo(utxo::Iter<'a, Address>),
    OldUtxo(utxo::Iter<'a, legacy::OldAddress>),
    Accounts(crate::accounting::account::Iter<'a, account::Identifier, ()>),
    ConfigParams(Vec<ConfigParam>),
    UpdateProposals(
        std::collections::btree_map::Iter<
            'a,
            crate::update::UpdateProposalId,
            crate::update::UpdateProposalState,
        >,
    ),
    MultisigAccounts(crate::accounting::account::Iter<'a, crate::multisig::Identifier, ()>),
    MultisigDeclarations(
        imhamt::HamtIter<'a, crate::multisig::Identifier, crate::multisig::Declaration>,
    ),
    StakePools(imhamt::HamtIter<'a, crate::stake::StakePoolId, crate::stake::StakePoolInfo>),
    Done,
}

pub struct LedgerIterator<'a> {
    ledger: &'a Ledger,
    state: IterState<'a>,
}

impl<'a> Iterator for LedgerIterator<'a> {
    type Item = Entry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.state {
            IterState::Initial => {
                self.state = IterState::Utxo(self.ledger.utxos.iter());
                Some(Entry::Globals(Globals {
                    date: self.ledger.date,
                    chain_length: self.ledger.chain_length,
                    static_params: (*self.ledger.static_params).clone(),
                    era: self.ledger.era.clone(),
                }))
            }
            IterState::Utxo(iter) => match iter.next() {
                None => {
                    self.state = IterState::OldUtxo(self.ledger.oldutxos.iter());
                    self.next()
                }
                Some(x) => Some(Entry::Utxo(x)),
            },
            IterState::OldUtxo(iter) => match iter.next() {
                None => {
                    self.state = IterState::Accounts(self.ledger.accounts.iter());
                    self.next()
                }
                Some(x) => Some(Entry::OldUtxo(x)),
            },
            IterState::Accounts(iter) => match iter.next() {
                None => {
                    self.state = IterState::ConfigParams(self.ledger.settings.to_config_params().0);
                    self.next()
                }
                Some(x) => Some(Entry::Account(x)),
            },
            IterState::ConfigParams(params) => {
                if let Some(param) = params.pop() {
                    Some(Entry::ConfigParam(param))
                } else {
                    self.state = IterState::UpdateProposals(self.ledger.updates.proposals.iter());
                    self.next()
                }
            }
            IterState::UpdateProposals(iter) => match iter.next() {
                None => {
                    self.state = IterState::MultisigAccounts(self.ledger.multisig.iter_accounts());
                    self.next()
                }
                Some(x) => Some(Entry::UpdateProposal(x)),
            },
            IterState::MultisigAccounts(iter) => match iter.next() {
                None => {
                    self.state =
                        IterState::MultisigDeclarations(self.ledger.multisig.iter_declarations());
                    self.next()
                }
                Some(x) => Some(Entry::MultisigAccount(x)),
            },
            IterState::MultisigDeclarations(iter) => match iter.next() {
                None => {
                    self.state = IterState::StakePools(self.ledger.delegation.stake_pools.iter());
                    self.next()
                }
                Some(x) => Some(Entry::MultisigDeclaration(x)),
            },
            IterState::StakePools(iter) => match iter.next() {
                None => {
                    self.state = IterState::Done;
                    self.next()
                }
                Some(x) => Some(Entry::StakePool(x)),
            },
            IterState::Done => None,
        }
    }
}

impl Ledger {
    pub fn iter<'a>(&'a self) -> LedgerIterator<'a> {
        LedgerIterator {
            ledger: self,
            state: IterState::Initial,
        }
    }
}

impl<'a> std::iter::FromIterator<Entry<'a>> for Result<Ledger, Error> {
    fn from_iter<I: IntoIterator<Item = Entry<'a>>>(iter: I) -> Self {
        let mut utxos = std::collections::HashMap::new();
        let mut oldutxos = std::collections::HashMap::new();
        let mut accounts = vec![];
        let mut config_params = crate::fragment::ConfigParams::new();
        let mut updates = update::UpdateState::new();
        let mut multisig_accounts = vec![];
        let mut multisig_declarations = vec![];
        let delegation = DelegationState::new();
        let mut globals = None;

        for entry in iter {
            match entry {
                Entry::Globals(globals2) => {
                    globals = Some(globals2);
                    // FIXME: check duplicate
                }
                Entry::Utxo(entry) => {
                    utxos
                        .entry(entry.fragment_id)
                        .or_insert(vec![])
                        .push((entry.output_index, entry.output.clone()));
                }
                Entry::OldUtxo(entry) => {
                    oldutxos
                        .entry(entry.fragment_id)
                        .or_insert(vec![])
                        .push((entry.output_index, entry.output.clone()));
                }
                Entry::Account((account_id, account_state)) => {
                    accounts.push((account_id.clone(), account_state.clone()));
                }
                Entry::ConfigParam(param) => {
                    config_params.push(param.clone());
                }
                Entry::UpdateProposal((proposal_id, proposal_state)) => {
                    updates
                        .proposals
                        .insert(proposal_id.clone(), proposal_state.clone());
                }
                Entry::MultisigAccount((account_id, account_state)) => {
                    multisig_accounts.push((account_id.clone(), account_state.clone()));
                }
                Entry::MultisigDeclaration((id, decl)) => {
                    multisig_declarations.push((id.clone(), decl.clone()));
                }
                Entry::StakePool((pool_id, pool_state)) => {
                    delegation
                        .stake_pools
                        .insert(pool_id.clone(), pool_state.clone())
                        .unwrap();
                }
            }
        }

        let globals = globals.ok_or(Error::IncompleteLedger)?;

        Ok(Ledger {
            utxos: utxos.into_iter().collect(),
            oldutxos: oldutxos.into_iter().collect(),
            accounts: accounts.into_iter().collect(),
            settings: setting::Settings::new().apply(&config_params)?,
            updates,
            multisig: multisig::Ledger::restore(multisig_accounts, multisig_declarations),
            delegation,
            static_params: Arc::new(globals.static_params),
            date: globals.date,
            chain_length: globals.chain_length,
            era: globals.era,
            pot: Value::zero(),
        })
    }
}
