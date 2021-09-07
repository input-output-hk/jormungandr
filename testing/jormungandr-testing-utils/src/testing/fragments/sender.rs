use super::{FragmentExporter, FragmentExporterError};
use crate::{
    stake_pool::StakePool,
    testing::{
        ensure_node_is_in_sync_with_others,
        fragments::node::{FragmentNode, MemPoolCheck},
        FragmentSenderSetup, FragmentVerifier, SyncNode, SyncNodeError, SyncWaitParams,
    },
    wallet::Wallet,
};
use chain_core::property::Fragment as _;
use chain_impl_mockchain::{
    block::BlockDate,
    certificate::{DecryptedPrivateTally, VotePlan, VoteTallyPayload},
    fee::LinearFee,
    fragment::Fragment,
    vote::Choice,
};
use jormungandr_lib::interfaces::{Address, FragmentsProcessingSummary};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{FragmentStatus, SettingsDto, Value},
    time::SystemTime,
};
use std::time::Duration;

#[derive(custom_debug::Debug, thiserror::Error)]
pub enum FragmentSenderError {
    #[error("fragment sent to node: {alias} is not in block due to '{reason}'")]
    FragmentNotInBlock {
        alias: String,
        reason: String,
        #[debug(skip)]
        logs: Vec<String>,
    },
    #[error(
        "Too many attempts failed ({attempts}) while trying to send fragment to node: {alias}"
    )]
    TooManyAttemptsFailed { attempts: u8, alias: String },
    #[error("fragment verifier error")]
    FragmentVerifierError(#[from] super::FragmentVerifierError),
    #[error("cannot send fragment")]
    SendFragmentError(#[from] super::node::FragmentNodeError),
    #[error("cannot sync node before sending fragment")]
    SyncNodeError(#[from] crate::testing::SyncNodeError),
    #[error("wallet error")]
    WalletError(#[from] crate::wallet::WalletError),
    #[error("wrong sender configuration: cannot use disable transaction auto confirm when sending more than one transaction")]
    TransactionAutoConfirmDisabledError,
    #[error("fragment exporter error")]
    FragmentExporterError(#[from] FragmentExporterError),
}

impl FragmentSenderError {
    pub fn logs(&self) -> impl Iterator<Item = &str> {
        use self::FragmentSenderError::*;
        let maybe_logs = match self {
            FragmentNotInBlock { logs, .. } => Some(logs),
            _ => None,
        };
        maybe_logs
            .into_iter()
            .map(|logs| logs.iter())
            .flatten()
            .map(String::as_str)
    }
}

#[derive(Clone)]
pub struct FragmentSender<'a, S: SyncNode + Send> {
    block0_hash: Hash,
    fees: LinearFee,
    setup: FragmentSenderSetup<'a, S>,
    expiry_generator: BlockDateGenerator,
}

impl<'a, S: SyncNode + Send> FragmentSender<'a, S> {
    pub fn new(
        block0_hash: Hash,
        fees: LinearFee,
        expiry_generator: BlockDateGenerator,
        setup: FragmentSenderSetup<'a, S>,
    ) -> Self {
        Self {
            block0_hash,
            fees,
            setup,
            expiry_generator,
        }
    }

    pub fn block0_hash(&self) -> Hash {
        self.block0_hash
    }

    pub fn fees(&self) -> LinearFee {
        self.fees
    }

    pub fn date(&self) -> BlockDate {
        self.expiry_generator.block_date()
    }

    pub fn set_valid_until(self, valid_until: BlockDate) -> Self {
        Self {
            expiry_generator: BlockDateGenerator::Fixed(valid_until),
            ..self
        }
    }

    pub fn clone_with_setup<U: SyncNode + Send>(
        &self,
        setup: FragmentSenderSetup<'a, U>,
    ) -> FragmentSender<'a, U> {
        FragmentSender {
            fees: self.fees(),
            block0_hash: self.block0_hash(),
            expiry_generator: self.expiry_generator.clone(),
            setup,
        }
    }

    pub fn send_batch_fragments<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        fragments: Vec<Fragment>,
        fail_fast: bool,
        node: &A,
    ) -> Result<FragmentsProcessingSummary, FragmentSenderError> {
        self.wait_for_node_sync_if_enabled(node)
            .map_err(FragmentSenderError::SyncNodeError)?;
        node.send_batch_fragments(fragments, fail_fast)
            .map_err(|e| e.into())
    }

    pub fn send_transaction<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        from: &mut Wallet,
        to: &Wallet,
        via: &A,
        value: Value,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        let address = to.address();
        let fragment = from.transaction_to(
            &self.block0_hash,
            &self.fees,
            self.expiry_generator.block_date(),
            address,
            value,
        )?;
        self.dump_fragment_if_enabled(from, &fragment, via)?;
        self.send_fragment(from, fragment, via)
    }

    pub fn send_transaction_to_many<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        from: &mut Wallet,
        to: &[Wallet],
        via: &A,
        value: Value,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        let addresses: Vec<Address> = to.iter().map(|x| x.address()).collect();
        let fragment = from.transaction_to_many(
            &self.block0_hash,
            &self.fees,
            self.expiry_generator.block_date(),
            &addresses,
            value,
        )?;
        self.dump_fragment_if_enabled(from, &fragment, via)?;
        self.send_fragment(from, fragment, via)
    }

    pub fn send_full_delegation<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        from: &mut Wallet,
        to: &StakePool,
        via: &A,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        let fragment = from.issue_full_delegation_cert(
            &self.block0_hash,
            &self.fees,
            self.expiry_generator.block_date(),
            to,
        )?;
        self.dump_fragment_if_enabled(from, &fragment, via)?;
        self.send_fragment(from, fragment, via)
    }

    pub fn send_split_delegation<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        from: &mut Wallet,
        distribution: &[(&StakePool, u8)],
        via: &A,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        let fragment = from.issue_split_delegation_cert(
            &self.block0_hash,
            &self.fees,
            self.expiry_generator.block_date(),
            distribution.to_vec(),
        )?;
        self.dump_fragment_if_enabled(from, &fragment, via)?;
        self.send_fragment(from, fragment, via)
    }

    pub fn send_owner_delegation<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        from: &mut Wallet,
        to: &StakePool,
        via: &A,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        let fragment = from.issue_owner_delegation_cert(
            &self.block0_hash,
            &self.fees,
            self.expiry_generator.block_date(),
            to,
        )?;
        self.dump_fragment_if_enabled(from, &fragment, via)?;
        self.send_fragment(from, fragment, via)
    }

    pub fn send_pool_registration<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        from: &mut Wallet,
        to: &StakePool,
        via: &A,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        let fragment = from.issue_pool_registration_cert(
            &self.block0_hash,
            &self.fees,
            self.expiry_generator.block_date(),
            to,
        )?;
        self.dump_fragment_if_enabled(from, &fragment, via)?;
        self.send_fragment(from, fragment, via)
    }

    pub fn send_pool_update<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        from: &mut Wallet,
        to: &StakePool,
        update_stake_pool: &StakePool,
        via: &A,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        let fragment = from.issue_pool_update_cert(
            &self.block0_hash,
            &self.fees,
            self.expiry_generator.block_date(),
            to,
            update_stake_pool,
        )?;
        self.dump_fragment_if_enabled(from, &fragment, via)?;
        self.send_fragment(from, fragment, via)
    }

    pub fn send_pool_retire<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        from: &mut Wallet,
        to: &StakePool,
        via: &A,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        let fragment = from.issue_pool_retire_cert(
            &self.block0_hash,
            &self.fees,
            self.expiry_generator.block_date(),
            to,
        )?;
        self.dump_fragment_if_enabled(from, &fragment, via)?;
        self.send_fragment(from, fragment, via)
    }

    pub fn send_vote_plan<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        from: &mut Wallet,
        vote_plan: &VotePlan,
        via: &A,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        let fragment = from.issue_vote_plan_cert(
            &self.block0_hash,
            &self.fees,
            self.expiry_generator.block_date(),
            vote_plan,
        )?;
        self.dump_fragment_if_enabled(from, &fragment, via)?;
        self.send_fragment(from, fragment, via)
    }

    pub fn send_vote_cast<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        from: &mut Wallet,
        vote_plan: &VotePlan,
        proposal_index: u8,
        choice: &Choice,
        via: &A,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        let fragment = from.issue_vote_cast_cert(
            &self.block0_hash,
            &self.fees,
            self.expiry_generator.block_date(),
            vote_plan,
            proposal_index,
            choice,
        )?;
        self.dump_fragment_if_enabled(from, &fragment, via)?;
        self.send_fragment(from, fragment, via)
    }

    pub fn send_public_vote_tally<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        from: &mut Wallet,
        vote_plan: &VotePlan,
        via: &A,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        self.send_vote_tally(from, vote_plan, via, VoteTallyPayload::Public)
    }

    pub fn send_encrypted_tally<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        from: &mut Wallet,
        vote_plan: &VotePlan,
        via: &A,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        let fragment = from.issue_encrypted_tally_cert(
            &self.block0_hash,
            &self.fees,
            self.expiry_generator.block_date(),
            vote_plan,
        )?;
        self.dump_fragment_if_enabled(from, &fragment, via)?;
        self.send_fragment(from, fragment, via)
    }

    pub fn send_private_vote_tally<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        from: &mut Wallet,
        vote_plan: &VotePlan,
        inner: DecryptedPrivateTally,
        via: &A,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        self.send_vote_tally(from, vote_plan, via, VoteTallyPayload::Private { inner })
    }

    pub fn send_vote_tally<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        from: &mut Wallet,
        vote_plan: &VotePlan,
        via: &A,
        tally_type: VoteTallyPayload,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        let fragment = from.issue_vote_tally_cert(
            &self.block0_hash,
            &self.fees,
            self.expiry_generator.block_date(),
            vote_plan,
            tally_type,
        )?;
        self.dump_fragment_if_enabled(from, &fragment, via)?;
        self.send_fragment(from, fragment, via)
    }

    pub fn send_transactions<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        n: u32,
        mut wallet1: &mut Wallet,
        wallet2: &Wallet,
        node: &A,
        value: Value,
    ) -> Result<(), FragmentSenderError> {
        if !self.setup.auto_confirm() {
            return Err(FragmentSenderError::TransactionAutoConfirmDisabledError);
        }

        for _ in 0..n {
            self.send_transaction(&mut wallet1, wallet2, node, value)?;
        }
        Ok(())
    }

    pub fn send_transactions_with_iteration_delay<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        n: u32,
        mut wallet1: &mut Wallet,
        wallet2: &mut Wallet,
        node: &A,
        value: Value,
        duration: Duration,
    ) -> Result<(), FragmentSenderError> {
        if !self.setup.auto_confirm() {
            return Err(FragmentSenderError::TransactionAutoConfirmDisabledError);
        }

        for _ in 0..n {
            self.send_transaction(&mut wallet1, wallet2, node, value)?;
            std::thread::sleep(duration);
        }
        Ok(())
    }

    pub fn send_transactions_round_trip<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        n: u32,
        mut wallet1: &mut Wallet,
        mut wallet2: &mut Wallet,
        node: &A,
        value: Value,
    ) -> Result<(), FragmentSenderError> {
        if !self.setup.auto_confirm() {
            return Err(FragmentSenderError::TransactionAutoConfirmDisabledError);
        }

        for _ in 0..n {
            self.send_transaction(&mut wallet1, wallet2, node, value)?;
            self.send_transaction(&mut wallet2, wallet1, node, value)?;
        }
        Ok(())
    }

    pub fn verify<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        check: &MemPoolCheck,
        node: &A,
    ) -> Result<(), FragmentSenderError> {
        match FragmentVerifier::wait_fragment(
            Duration::from_secs(2),
            check.clone(),
            Default::default(),
            node,
        )? {
            FragmentStatus::Rejected { reason } => Err(FragmentSenderError::FragmentNotInBlock {
                alias: FragmentNode::alias(node).to_string(),
                reason,
                logs: FragmentNode::log_content(node),
            }),
            FragmentStatus::InABlock { .. } => Ok(()),
            _ => unimplemented!(),
        }
    }

    fn dump_fragment_if_enabled(
        &self,
        sender: &Wallet,
        fragment: &Fragment,
        via: &dyn FragmentNode,
    ) -> Result<(), FragmentSenderError> {
        if let Some(dump_folder) = &self.setup.dump_fragments {
            FragmentExporter::new(dump_folder.to_path_buf())?
                .dump_to_file(fragment, sender, via)?;
        }
        Ok(())
    }

    pub fn send_fragment<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        sender: &mut Wallet,
        fragment: Fragment,
        node: &A,
    ) -> Result<MemPoolCheck, FragmentSenderError> {
        self.wait_for_node_sync_if_enabled(node)
            .map_err(FragmentSenderError::SyncNodeError)?;
        for _ in 0..self.setup.attempts_count() {
            let check = node.send_fragment(fragment.clone());

            if self.setup.fire_and_forget() {
                self.confirm_transaction_if_enabled(sender);
                return Ok(MemPoolCheck::new(fragment.id()));
            }

            if let Err(send_fragment_error) = check {
                if self.setup.ignore_any_errors() {
                    return Ok(MemPoolCheck::new(fragment.id()));
                }
                return Err(FragmentSenderError::SendFragmentError(send_fragment_error));
            }

            if let Err(err) = self.verify(&check.unwrap(), node) {
                if self.setup.ignore_any_errors() {
                    println!("Ignoring error: {:?}", err);
                    return Ok(MemPoolCheck::new(fragment.id()));
                }
                println!(
                    "Error while sending fragment {:?}. Retrying if possible...",
                    err
                );
                continue;
            }
            self.confirm_transaction_if_enabled(sender);
            return Ok(MemPoolCheck::new(fragment.id()));
        }

        if self.setup.ignore_any_errors() {
            self.confirm_transaction_if_enabled(sender);
            return Ok(MemPoolCheck::new(fragment.id()));
        }

        Err(FragmentSenderError::TooManyAttemptsFailed {
            attempts: self.setup.attempts_count(),
            alias: FragmentNode::alias(node).to_string(),
        })
    }

    fn confirm_transaction_if_enabled(&self, sender: &mut Wallet) {
        if self.setup.auto_confirm() {
            sender.confirm_transaction();
        }
    }

    fn wait_for_node_sync_if_enabled<A: FragmentNode + SyncNode + Sized + Send>(
        &self,
        node: &A,
    ) -> Result<(), SyncNodeError> {
        if self.setup.no_sync_nodes() {
            return Ok(());
        }

        let nodes_length = (self.setup.sync_nodes().len() + 1) as u64;
        ensure_node_is_in_sync_with_others(
            node,
            self.setup.sync_nodes(),
            SyncWaitParams::network_size(nodes_length, 2).into(),
            "waiting for node to be in sync before sending transaction",
        )
    }
}

#[derive(Clone)]
pub enum BlockDateGenerator {
    Rolling {
        block0_time: SystemTime,
        slot_duration: u64,
        slots_per_epoch: u32,
        shift: BlockDate,
        shift_back: bool,
    },
    Fixed(BlockDate),
}

impl BlockDateGenerator {
    /// Returns `BlockDate`s that are always ahead or behind the current date by a certain shift
    pub fn rolling(block0_settings: &SettingsDto, shift: BlockDate, shift_back: bool) -> Self {
        Self::Rolling {
            block0_time: block0_settings.block0_time,
            slot_duration: block0_settings.slot_duration,
            slots_per_epoch: block0_settings.slots_per_epoch,
            shift,
            shift_back,
        }
    }

    /// Shifts the current date and returns the result on all subsequent calls
    pub fn shifted(block0_settings: &SettingsDto, shift: BlockDate, shift_back: bool) -> Self {
        let current = Self::current_blockchain_age(
            block0_settings.block0_time,
            block0_settings.slots_per_epoch,
            block0_settings.slot_duration,
        );

        let shifted = if shift_back {
            Self::shift_back(block0_settings.slots_per_epoch, current, shift)
        } else {
            Self::shift_ahead(block0_settings.slots_per_epoch, current, shift)
        };

        Self::Fixed(shifted)
    }

    pub fn block_date(&self) -> BlockDate {
        match self {
            Self::Fixed(valid_until) => *valid_until,
            Self::Rolling {
                block0_time,
                slot_duration,
                slots_per_epoch,
                shift,
                shift_back,
            } => {
                let current =
                    Self::current_blockchain_age(*block0_time, *slots_per_epoch, *slot_duration);

                if *shift_back {
                    Self::shift_back(*slots_per_epoch, current, *shift)
                } else {
                    Self::shift_ahead(*slots_per_epoch, current, *shift)
                }
            }
        }
    }

    pub fn shift_ahead(slots_per_epoch: u32, date: BlockDate, shift: BlockDate) -> BlockDate {
        if shift.slot_id >= slots_per_epoch {
            panic!(
                "Requested shift by {} but an epoch has {} slots",
                shift, slots_per_epoch
            );
        }

        let epoch = date.epoch + shift.epoch + (date.slot_id + shift.slot_id) / slots_per_epoch;
        let slot_id = (date.slot_id + shift.slot_id) % slots_per_epoch;

        BlockDate { epoch, slot_id }
    }

    pub fn shift_back(slots_per_epoch: u32, date: BlockDate, shift: BlockDate) -> BlockDate {
        if shift.slot_id >= slots_per_epoch {
            panic!(
                "Requested shift by -{} but an epoch has {} slots",
                shift, slots_per_epoch
            );
        }

        let epoch = date.epoch - shift.epoch - (date.slot_id + shift.slot_id) / slots_per_epoch;
        let slot_id = (date.slot_id + shift.slot_id) % slots_per_epoch;

        BlockDate { epoch, slot_id }
    }

    pub fn current_blockchain_age(
        block0_time: SystemTime,
        slots_per_epoch: u32,
        slot_duration: u64,
    ) -> BlockDate {
        let now = SystemTime::now();

        let slots_since_block0 = now
            .duration_since(block0_time)
            .unwrap_or_else(|_| jormungandr_lib::time::Duration::from_millis(0))
            .as_secs()
            / slot_duration;

        let epoch = slots_since_block0 / slots_per_epoch as u64;
        let slot_id = slots_since_block0 % slots_per_epoch as u64;

        BlockDate {
            epoch: epoch as u32,
            slot_id: slot_id as u32,
        }
    }
}
