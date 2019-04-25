//! Mockchain ledger. Ledger exists in order to update the
//! current state and verify transactions.

use crate::block::{ChainLength, ConsensusVersion, HeaderHash};
use crate::config::{self, Block0Date, ConfigParam};
use crate::fee::LinearFee;
use crate::leadership::bft::LeaderId;
use crate::message::Message;
use crate::stake::{DelegationError, DelegationState, StakeDistribution};
use crate::transaction::*;
use crate::value::*;
use crate::{account, certificate, legacy, setting, stake, utxo};
use chain_addr::{Address, Discrimination, Kind};
use chain_core::property::{self, ChainLength as _, Message as _};
use std::sync::Arc;

// static parameters, effectively this is constant in the parameter of the blockchain
#[derive(Clone)]
pub struct LedgerStaticParameters {
    pub block0_initial_hash: HeaderHash,
    pub block0_start_time: config::Block0Date,
    pub block0_consensus: ConsensusVersion,
    pub discrimination: Discrimination,
}

// parameters to validate ledger
#[derive(Clone)]
pub struct LedgerParameters {
    pub fees: LinearFee,
    pub allow_account_creation: bool,
}

/// Overall ledger structure.
///
/// This represent a given state related to utxo/old utxo/accounts/... at a given
/// point in time.
///
/// The ledger can be easily and cheaply cloned despite containing reference
/// to a lot of data (millions of utxos, thousands of accounts, ..)
#[derive(Clone)]
pub struct Ledger {
    pub(crate) utxos: utxo::Ledger<Address>,
    pub(crate) oldutxos: utxo::Ledger<legacy::OldAddress>,
    pub(crate) accounts: account::Ledger,
    pub(crate) settings: setting::Settings,
    pub(crate) updates: setting::UpdateState,
    pub(crate) delegation: DelegationState,
    pub(crate) static_params: Arc<LedgerStaticParameters>,
    pub(crate) chain_length: ChainLength,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Config(config::Error),
    NotEnoughSignatures(usize, usize),
    UtxoValueNotMatching(Value, Value),
    UtxoError(utxo::Error),
    UtxoInvalidSignature(UtxoPointer, Output<Address>, Witness),
    OldUtxoInvalidSignature(UtxoPointer, Output<legacy::OldAddress>, Witness),
    OldUtxoInvalidPublicKey(UtxoPointer, Output<legacy::OldAddress>, Witness),
    AccountInvalidSignature(account::Identifier, Witness),
    TransactionHasNoInput,
    Block0OnlyMessageReceived,
    Block0TransactionHasInput,
    Block0TransactionHasOutput,
    Block0TransactionHasWitnesses,
    Block0InitialMessageMissing,
    Block0InitialMessageDuplicateBlock0Date,
    Block0InitialMessageDuplicateDiscrimination,
    Block0InitialMessageDuplicateConsensusVersion,
    Block0InitialMessageDuplicateSlotDuration,
    Block0InitialMessageDuplicateEpochStabilityDepth,
    Block0InitialMessageNoBlock0Date,
    Block0InitialMessageNoDiscrimination,
    Block0InitialMessageNoConsensusVersion,
    Block0InitialMessageNoSlotDuration,
    Block0InitialMessageNoConsensusLeaderId,
    Block0UtxoTotalValueTooBig,
    Block0HasUpdateVote,
    UtxoInputsTotal(ValueError),
    UtxoOutputsTotal(ValueError),
    Account(account::LedgerError),
    NotBalanced(Value, Value),
    ZeroOutput(Output<Address>),
    Delegation(DelegationError),
    InvalidDiscrimination,
    ExpectingAccountWitness,
    ExpectingUtxoWitness,
    ExpectingInitialMessage,
    CertificateInvalidSignature,
    Update(setting::Error),
}

impl From<utxo::Error> for Error {
    fn from(e: utxo::Error) -> Self {
        Error::UtxoError(e)
    }
}

impl From<account::LedgerError> for Error {
    fn from(e: account::LedgerError) -> Self {
        Error::Account(e)
    }
}

impl From<DelegationError> for Error {
    fn from(e: DelegationError) -> Self {
        Error::Delegation(e)
    }
}

impl From<config::Error> for Error {
    fn from(e: config::Error) -> Self {
        Error::Config(e)
    }
}

impl From<setting::Error> for Error {
    fn from(e: setting::Error) -> Self {
        Error::Update(e)
    }
}

impl Ledger {
    pub fn new<'a, I>(block0_hash: HeaderHash, contents: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = &'a Message>,
    {
        let mut content_iter = contents.into_iter();
        let mut ledger_params = LedgerParameters {
            fees: LinearFee::new(0, 0, 0),
            allow_account_creation: false,
        };

        let init_ents = match content_iter.next() {
            Some(Message::Initial(ref init_ents)) => Ok(init_ents),
            Some(_) => Err(Error::ExpectingInitialMessage),
            None => Err(Error::Block0InitialMessageMissing),
        }?;
        let mut ledger = init_ents
            .iter()
            .try_fold(
                Default::default(),
                EmptyLedgerBuilder::try_with_config_param,
            )?
            .build(block0_hash)?;

        for content in content_iter {
            match content {
                Message::Initial(_) => {
                    return Err(Error::Block0InitialMessageMissing);
                }
                Message::OldUtxoDeclaration(old) => {
                    ledger.oldutxos = apply_old_declaration(ledger.oldutxos, old)?;
                }
                Message::Transaction(authenticated_tx) => {
                    if authenticated_tx.transaction.inputs.len() != 0 {
                        return Err(Error::Block0TransactionHasInput);
                    }
                    if authenticated_tx.witnesses.len() != 0 {
                        return Err(Error::Block0TransactionHasWitnesses);
                    }
                    let transaction_id = authenticated_tx.transaction.hash();
                    let (new_utxos, new_accounts) = internal_apply_transaction_output(
                        ledger.utxos,
                        ledger.accounts,
                        &ledger.static_params,
                        &ledger_params,
                        &transaction_id,
                        &authenticated_tx.transaction.outputs,
                    )?;
                    ledger.utxos = new_utxos;
                    ledger.accounts = new_accounts;;
                }
                Message::UpdateProposal(update_proposal) => {
                    // We assume here that the initial block contains
                    // a single update proposal with the initial
                    // settings, which we apply immediately without
                    // requiring any votes. FIXME: check the
                    // signature? Doesn't really matter, we have to
                    // trust block 0 anyway.
                    ledger = ledger.apply_update(&update_proposal.proposal.proposal)?;
                    ledger_params = ledger.get_ledger_parameters();
                }
                Message::UpdateVote(_) => {
                    return Err(Error::Block0HasUpdateVote);
                }
                Message::Certificate(authenticated_cert_tx) => {
                    if authenticated_cert_tx.transaction.inputs.len() != 0 {
                        return Err(Error::Block0TransactionHasInput);
                    }
                    if authenticated_cert_tx.witnesses.len() != 0 {
                        return Err(Error::Block0TransactionHasWitnesses);
                    }
                    if authenticated_cert_tx.transaction.outputs.len() != 0 {
                        return Err(Error::Block0TransactionHasOutput);
                    }
                    ledger.delegation = ledger
                        .delegation
                        .apply(&authenticated_cert_tx.transaction.extra)?;
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
    ) -> Result<Self, Error>
    where
        I: IntoIterator<Item = &'a Message>,
    {
        let mut new_ledger = self.clone();

        new_ledger.chain_length = self.chain_length.next();

        // If we entered a new epoch, then delete expired update
        // proposals and apply accepted update proposals.
        // FIXME: do this at an epoch boundary; need to know current date.
        let (updates, settings) = new_ledger.updates.process_proposals(new_ledger.settings);
        new_ledger.updates = updates;
        new_ledger.settings = settings;

        for content in contents {
            match content {
                Message::Initial(_) => return Err(Error::Block0OnlyMessageReceived),
                Message::OldUtxoDeclaration(_) => return Err(Error::Block0OnlyMessageReceived),
                Message::Transaction(authenticated_tx) => {
                    new_ledger = new_ledger.apply_transaction(&authenticated_tx, &ledger_params)?;
                }
                Message::UpdateProposal(update_proposal) => {
                    new_ledger =
                        new_ledger.apply_update_proposal(content.id(), &update_proposal)?;
                }
                Message::UpdateVote(vote) => {
                    new_ledger = new_ledger.apply_update_vote(&vote)?;
                }
                Message::Certificate(authenticated_cert_tx) => {
                    new_ledger =
                        new_ledger.apply_certificate(authenticated_cert_tx, &ledger_params)?;
                }
            }
        }
        Ok(new_ledger)
    }

    pub fn apply_transaction<Extra: property::Serialize>(
        mut self,
        signed_tx: &AuthenticatedTransaction<Address, Extra>,
        dyn_params: &LedgerParameters,
    ) -> Result<Self, Error> {
        let transaction_id = signed_tx.transaction.hash();
        self = internal_apply_transaction(
            self,
            dyn_params,
            &transaction_id,
            &signed_tx.transaction.inputs[..],
            &signed_tx.transaction.outputs[..],
            &signed_tx.witnesses[..],
        )?;
        Ok(self)
    }

    pub fn apply_update(mut self, update: &setting::UpdateProposal) -> Result<Self, Error> {
        self.settings = self.settings.apply(update);
        Ok(self)
    }

    pub fn apply_update_proposal(
        mut self,
        proposal_id: setting::UpdateProposalId,
        proposal: &setting::SignedUpdateProposal,
    ) -> Result<Self, Error> {
        self.updates = self
            .updates
            .apply_proposal(proposal_id, proposal, &self.settings)?;
        Ok(self)
    }

    pub fn apply_update_vote(mut self, vote: &setting::SignedUpdateVote) -> Result<Self, Error> {
        self.updates = self.updates.apply_vote(vote, &self.settings)?;
        Ok(self)
    }

    pub fn apply_certificate(
        mut self,
        auth_cert: &AuthenticatedTransaction<Address, certificate::Certificate>,
        dyn_params: &LedgerParameters,
    ) -> Result<Self, Error> {
        let verified = auth_cert.transaction.extra.verify();
        if verified == chain_crypto::Verification::Failed {
            return Err(Error::CertificateInvalidSignature);
        };
        self = self.apply_transaction(auth_cert, dyn_params)?;
        self.delegation = self.delegation.apply(&auth_cert.transaction.extra)?;
        Ok(self)
    }

    pub fn get_stake_distribution(&self) -> StakeDistribution {
        stake::get_distribution(&self.delegation, &self.utxos)
    }

    /// access the ledger static parameters
    pub fn get_static_parameters(&self) -> &LedgerStaticParameters {
        self.static_params.as_ref()
    }

    pub fn get_ledger_parameters(&self) -> LedgerParameters {
        LedgerParameters {
            fees: *self.settings.linear_fees,
            allow_account_creation: self.settings.allow_account_creation,
        }
    }

    pub fn consensus_version(&self) -> ConsensusVersion {
        // TODO: this may be updated overtime (bft -> switch to genesis ?)
        self.static_params.block0_consensus
    }

    pub fn utxos<'a>(&'a self) -> utxo::Iter<'a, Address> {
        self.utxos.iter()
    }

    pub fn chain_length(&self) -> ChainLength {
        self.chain_length
    }

    fn validate_utxo_total_value(&self) -> Result<(), Error> {
        let old_utxo_values = self.oldutxos.iter().map(|entry| entry.output.value);
        let new_utxo_values = self.utxos.iter().map(|entry| entry.output.value);
        let account_value = self
            .accounts
            .get_total_value()
            .map_err(|_| Error::Block0UtxoTotalValueTooBig)?;
        let all_utxo_values = old_utxo_values
            .chain(new_utxo_values)
            .chain(Some(account_value));
        Value::sum(all_utxo_values).map_err(|_| Error::Block0UtxoTotalValueTooBig)?;
        Ok(())
    }
}

fn apply_old_declaration(
    mut utxos: utxo::Ledger<legacy::OldAddress>,
    decl: &legacy::UtxoDeclaration,
) -> Result<utxo::Ledger<legacy::OldAddress>, Error> {
    assert!(decl.addrs.len() < 255);
    let txid = decl.hash();
    let mut outputs = Vec::with_capacity(decl.addrs.len());
    for (i, d) in decl.addrs.iter().enumerate() {
        let output = Output {
            address: d.0.clone(),
            value: d.1,
        };
        outputs.push((i as u8, output))
    }
    utxos = utxos.add(&txid, &outputs)?;
    Ok(utxos)
}

/// Apply the transaction
fn internal_apply_transaction(
    mut ledger: Ledger,
    dyn_params: &LedgerParameters,
    transaction_id: &TransactionId,
    inputs: &[Input],
    outputs: &[Output<Address>],
    witnesses: &[Witness],
) -> Result<Ledger, Error> {
    assert!(inputs.len() < 255);
    assert!(outputs.len() < 255);
    assert!(witnesses.len() < 255);

    if inputs.len() == 0 {
        return Err(Error::TransactionHasNoInput);
    }

    // 1. verify that number of signatures matches number of
    // transactions
    if inputs.len() != witnesses.len() {
        return Err(Error::NotEnoughSignatures(inputs.len(), witnesses.len()));
    }

    // 2. validate inputs of transaction by gathering what we know of it,
    // then verifying the associated witness
    for (input, witness) in inputs.iter().zip(witnesses.iter()) {
        match input.to_enum() {
            InputEnum::UtxoInput(utxo) => {
                ledger = input_utxo_verify(ledger, transaction_id, &utxo, witness)?
            }
            InputEnum::AccountInput(account_id, value) => {
                ledger.accounts = input_account_verify(
                    ledger.accounts,
                    transaction_id,
                    &account_id,
                    value,
                    witness,
                )?
            }
        }
    }

    // 3. verify that transaction sum is zero.
    // TODO: with fees this will change
    let total_input =
        Value::sum(inputs.iter().map(|i| i.value)).map_err(|e| Error::UtxoInputsTotal(e))?;
    let total_output =
        Value::sum(inputs.iter().map(|i| i.value)).map_err(|e| Error::UtxoOutputsTotal(e))?;
    if total_input != total_output {
        return Err(Error::NotBalanced(total_input, total_output));
    }

    // 4. add the new outputs
    let (new_utxos, new_accounts) = internal_apply_transaction_output(
        ledger.utxos,
        ledger.accounts,
        &ledger.static_params,
        dyn_params,
        transaction_id,
        outputs,
    )?;
    ledger.utxos = new_utxos;
    ledger.accounts = new_accounts;

    Ok(ledger)
}

fn internal_apply_transaction_output(
    mut utxos: utxo::Ledger<Address>,
    mut accounts: account::Ledger,
    static_params: &LedgerStaticParameters,
    dyn_params: &LedgerParameters,
    transaction_id: &TransactionId,
    outputs: &[Output<Address>],
) -> Result<(utxo::Ledger<Address>, account::Ledger), Error> {
    let mut new_utxos = Vec::new();
    for (index, output) in outputs.iter().enumerate() {
        // Reject zero-valued outputs.
        if output.value == Value::zero() {
            return Err(Error::ZeroOutput(output.clone()));
        }

        if output.address.discrimination() != static_params.discrimination {
            return Err(Error::InvalidDiscrimination);
        }
        match output.address.kind() {
            Kind::Single(_) | Kind::Group(_, _) => {
                new_utxos.push((index as u8, output.clone()));
            }
            Kind::Account(identifier) => {
                // don't have a way to make a newtype ref from the ref so .clone()
                let account = identifier.clone().into();
                accounts = match accounts.add_value(&account, output.value) {
                    Ok(accounts) => accounts,
                    Err(account::LedgerError::NonExistent) if dyn_params.allow_account_creation => {
                        // if the account was not existent and that we allow creating
                        // account out of the blue, then fallback on adding the account
                        accounts.add_account(&account, output.value)?
                    }
                    Err(error) => return Err(error.into()),
                };
            }
        }
    }

    utxos = utxos.add(transaction_id, &new_utxos)?;
    Ok((utxos, accounts))
}

fn input_utxo_verify(
    mut ledger: Ledger,
    transaction_id: &TransactionId,
    utxo: &UtxoPointer,
    witness: &Witness,
) -> Result<Ledger, Error> {
    match witness {
        Witness::Account(_) => return Err(Error::ExpectingUtxoWitness),
        Witness::OldUtxo(xpub, signature) => {
            let (old_utxos, associated_output) = ledger
                .oldutxos
                .remove(&utxo.transaction_id, utxo.output_index)?;

            ledger.oldutxos = old_utxos;
            if utxo.value != associated_output.value {
                return Err(Error::UtxoValueNotMatching(
                    utxo.value,
                    associated_output.value,
                ));
            };

            if legacy::oldaddress_from_xpub(&associated_output.address, xpub) {
                return Err(Error::OldUtxoInvalidPublicKey(
                    utxo.clone(),
                    associated_output.clone(),
                    witness.clone(),
                ));
            };

            let verified = signature.verify(&xpub, &transaction_id);
            if verified == chain_crypto::Verification::Failed {
                return Err(Error::OldUtxoInvalidSignature(
                    utxo.clone(),
                    associated_output.clone(),
                    witness.clone(),
                ));
            };

            Ok(ledger)
        }
        Witness::Utxo(signature) => {
            let (new_utxos, associated_output) = ledger
                .utxos
                .remove(&utxo.transaction_id, utxo.output_index)?;
            ledger.utxos = new_utxos;
            if utxo.value != associated_output.value {
                return Err(Error::UtxoValueNotMatching(
                    utxo.value,
                    associated_output.value,
                ));
            }

            let verified = signature.verify(
                &associated_output.address.public_key().unwrap(),
                &transaction_id,
            );
            if verified == chain_crypto::Verification::Failed {
                return Err(Error::UtxoInvalidSignature(
                    utxo.clone(),
                    associated_output.clone(),
                    witness.clone(),
                ));
            };
            Ok(ledger)
        }
    }
}

fn input_account_verify(
    mut ledger: account::Ledger,
    transaction_id: &TransactionId,
    account: &account::Identifier,
    value: Value,
    witness: &Witness,
) -> Result<account::Ledger, Error> {
    // .remove_value() check if there's enough value and if not, returns a Err.
    let (new_ledger, spending_counter) = ledger.remove_value(account, value)?;
    ledger = new_ledger;

    match witness {
        Witness::OldUtxo(_, _) => return Err(Error::ExpectingAccountWitness),
        Witness::Utxo(_) => return Err(Error::ExpectingAccountWitness),
        Witness::Account(sig) => {
            let tidsc = TransactionIdSpendingCounter::new(transaction_id, &spending_counter);
            let verified = sig.verify(&account.clone().into(), &tidsc);
            if verified == chain_crypto::Verification::Failed {
                return Err(Error::AccountInvalidSignature(
                    account.clone(),
                    witness.clone(),
                ));
            };
            Ok(ledger)
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Default)]
struct EmptyLedgerBuilder {
    block0_date: Option<Block0Date>,
    discrimination: Option<Discrimination>,
    consensus_version: Option<ConsensusVersion>,
    slot_duration: Option<u8>,
    epoch_stability_depth: Option<u32>,
    consensus_leader_ids: Vec<LeaderId>,
}

impl EmptyLedgerBuilder {
    pub fn try_with_config_param(mut self, param: &ConfigParam) -> Result<Self, Error> {
        match param {
            ConfigParam::Block0Date(param) => self
                .block0_date
                .replace(*param)
                .map(|_| Error::Block0InitialMessageDuplicateBlock0Date),
            ConfigParam::Discrimination(param) => self
                .discrimination
                .replace(*param)
                .map(|_| Error::Block0InitialMessageDuplicateDiscrimination),
            ConfigParam::ConsensusVersion(param) => self
                .consensus_version
                .replace(*param)
                .map(|_| Error::Block0InitialMessageDuplicateConsensusVersion),
            ConfigParam::SlotDuration(param) => self
                .slot_duration
                .replace(*param)
                .map(|_| Error::Block0InitialMessageDuplicateSlotDuration),
            ConfigParam::EpochStabilityDepth(param) => self
                .epoch_stability_depth
                .replace(*param)
                .map(|_| Error::Block0InitialMessageDuplicateEpochStabilityDepth),
            ConfigParam::ConsensusLeaderId(param) => {
                self.consensus_leader_ids.push(param.clone());
                None
            }
            ConfigParam::SlotsPerEpoch(_)
            | ConfigParam::ConsensusGenesisPraosParamD(_)
            | ConfigParam::ConsensusGenesisPraosParamF(_) => None,
        }
        .map(|e| Err(e))
        .unwrap_or(Ok(self))
    }

    pub fn build(self, block0_initial_hash: HeaderHash) -> Result<Ledger, Error> {
        // generates warnings for each unused parameter
        let EmptyLedgerBuilder {
            block0_date,
            discrimination,
            consensus_version,
            slot_duration,
            epoch_stability_depth,
            consensus_leader_ids,
        } = self;

        let mut settings = setting::Settings::new();
        if let Some(slot_duration) = slot_duration {
            settings.slot_duration = slot_duration;
        }
        if let Some(epoch_stability_depth) = epoch_stability_depth {
            settings.epoch_stability_depth = epoch_stability_depth;
        }
        match consensus_leader_ids.len() {
            0 => return Err(Error::Block0InitialMessageNoConsensusLeaderId),
            _ => settings.bft_leaders = Arc::new(consensus_leader_ids),
        }

        let static_params = LedgerStaticParameters {
            block0_initial_hash,
            block0_start_time: block0_date.ok_or(Error::Block0InitialMessageNoBlock0Date)?,
            block0_consensus: consensus_version
                .ok_or(Error::Block0InitialMessageNoConsensusVersion)?,
            discrimination: discrimination.ok_or(Error::Block0InitialMessageNoDiscrimination)?,
        };

        Ok(Ledger {
            utxos: utxo::Ledger::new(),
            oldutxos: utxo::Ledger::new(),
            accounts: account::Ledger::new(),
            settings,
            updates: setting::UpdateState::new(),
            delegation: DelegationState::new(),
            static_params: Arc::new(static_params),
            chain_length: ChainLength(0),
        })
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::key::{SpendingPublicKey, SpendingSecretKey};
    use crate::message::initial;
    use chain_addr::{Address, Discrimination, Kind};
    use chain_crypto::SecretKey;
    use rand::{CryptoRng, RngCore};

    pub fn make_key<R: RngCore + CryptoRng>(
        rng: &mut R,
        discrimination: &Discrimination,
    ) -> (SpendingSecretKey, SpendingPublicKey, Address) {
        let sk = SpendingSecretKey::generate(rng);
        let pk = sk.to_public();
        let user_address = Address(discrimination.clone(), Kind::Single(pk.clone()));
        (sk, pk, user_address)
    }

    macro_rules! assert_err {
        ($left: expr, $right: expr) => {
            match &($left) {
                left_val => match &($right) {
                    Err(e) => {
                        if !(e == left_val) {
                            panic!(
                                "assertion failed: error mismatch \
                                 (left: `{:?}, right: `{:?}`)",
                                *left_val, *e
                            )
                        }
                    }
                    Ok(_) => panic!(
                        "assertion failed: expected error {:?} but got success",
                        *left_val
                    ),
                },
            }
        };
    }

    #[test]
    pub fn utxo() -> () {
        let block0_hash = HeaderHash::hash_bytes(&[1, 2, 3]);
        let discrimination = Discrimination::Test;
        let mut ie = initial::InitialEnts::new();
        ie.push(ConfigParam::Discrimination(Discrimination::Test));
        ie.push(ConfigParam::ConsensusVersion(ConsensusVersion::Bft));
        let leader_pub_key = SecretKey::generate(rand::thread_rng()).to_public();
        ie.push(ConfigParam::ConsensusLeaderId(LeaderId::from(
            leader_pub_key,
        )));
        ie.push(ConfigParam::Block0Date(Block0Date(0)));

        let mut rng = rand::thread_rng();
        let (sk1, _pk1, user1_address) = make_key(&mut rng, &discrimination);
        let (_sk2, _pk2, user2_address) = make_key(&mut rng, &discrimination);
        let value = Value(42000);

        let output0 = Output {
            address: user1_address.clone(),
            value: value,
        };

        let first_trans = AuthenticatedTransaction {
            transaction: Transaction {
                inputs: vec![],
                outputs: vec![output0],
                extra: NoExtra,
            },
            witnesses: vec![],
        };
        let tx0_id = first_trans.transaction.hash();

        let utxo0 = UtxoPointer {
            transaction_id: tx0_id,
            output_index: 0,
            value: value,
        };

        let messages = [Message::Initial(ie), Message::Transaction(first_trans)];
        let ledger = Ledger::new(block0_hash, &messages).unwrap();
        let dyn_params = ledger.get_ledger_parameters();

        {
            let ledger = ledger.clone();
            let tx = Transaction {
                inputs: vec![Input::from_utxo(utxo0)],
                outputs: vec![Output {
                    address: user2_address.clone(),
                    value: Value(1),
                }],
                extra: NoExtra,
            };
            let signed_tx = AuthenticatedTransaction {
                transaction: tx,
                witnesses: vec![],
            };
            let r = ledger.apply_transaction(&signed_tx, &dyn_params);
            assert_err!(Error::NotEnoughSignatures(1, 0), r)
        }

        {
            let ledger = ledger.clone();
            let tx = Transaction {
                inputs: vec![Input::from_utxo(utxo0)],
                outputs: vec![Output {
                    address: user2_address.clone(),
                    value: Value(1),
                }],
                extra: NoExtra,
            };
            let txid = tx.hash();
            let w1 = Witness::new_utxo(&txid, &sk1);
            let signed_tx = AuthenticatedTransaction {
                transaction: tx,
                witnesses: vec![w1],
            };
            let r = ledger.apply_transaction(&signed_tx, &dyn_params);
            assert!(r.is_ok())
        }
    }
}
