use crate::{
    fragment::verifier::{FragmentVerifier, FragmentVerifierError},
    wallet::Wallet,
    BlockDateGenerator, FragmentSender, FragmentSenderError, FragmentSenderSetup,
};
use chain_impl_mockchain::{
    block::BlockDate,
    certificate::{VotePlan, VoteTallyPayload},
    fee::LinearFee,
    vote::Choice,
};
use jormungandr_automation::{
    jormungandr::{MemPoolCheck, RemoteJormungandr},
    testing::{time, SyncNode},
};
use jormungandr_lib::{crypto::hash::Hash, interfaces::Block0Configuration};

pub struct FragmentChainSender<'a, S: SyncNode + Send> {
    sender: FragmentSender<'a, S>,
    node: RemoteJormungandr,
    last_mempool_check: Option<MemPoolCheck>,
}

impl<'a, S: SyncNode + Send> FragmentChainSender<'a, S> {
    pub fn from_with_setup(
        block0_configuration: &Block0Configuration,
        node: RemoteJormungandr,
        setup: FragmentSenderSetup<'a, S>,
    ) -> Self {
        Self {
            sender: FragmentSender::from(block0_configuration).clone_with_setup(setup),
            node,
            last_mempool_check: None,
        }
    }

    pub fn new(
        block0_hash: Hash,
        fees: LinearFee,
        block_date_generator: BlockDateGenerator,
        setup: FragmentSenderSetup<'a, S>,
        node: RemoteJormungandr,
    ) -> Self {
        Self {
            sender: FragmentSender::new(block0_hash, fees, block_date_generator, setup),
            node,
            last_mempool_check: None,
        }
    }

    pub fn send_vote_plan(
        mut self,
        from: &mut Wallet,
        vote_plan: &VotePlan,
    ) -> Result<Self, FragmentChainSenderError> {
        self.last_mempool_check = Some(self.sender.send_vote_plan(from, vote_plan, &self.node)?);
        Ok(self)
    }

    pub fn and_verify_is_in_block(
        self,
        duration: std::time::Duration,
    ) -> Result<Self, FragmentChainSenderError> {
        FragmentVerifier::wait_and_verify_is_in_block(
            duration,
            self.get_last_mempool_check()?,
            &self.node,
        )?;
        Ok(self)
    }

    fn get_last_mempool_check(&self) -> Result<MemPoolCheck, FragmentChainSenderError> {
        self.last_mempool_check
            .clone()
            .ok_or(FragmentChainSenderError::NoFragmentToVerify)
    }

    pub fn then_wait_for_epoch(self, span: u32) -> Self {
        time::wait_for_epoch(span, self.node.rest().clone());
        let slot_id = self.sender.date().slot_id;
        Self {
            sender: self.sender.set_valid_until(BlockDate {
                epoch: span + 1,
                slot_id,
            }),
            ..self
        }
    }

    pub fn cast_vote(
        mut self,
        from: &mut Wallet,
        vote_plan: &VotePlan,
        proposal_index: u8,
        choice: &Choice,
    ) -> Result<Self, FragmentChainSenderError> {
        self.last_mempool_check = Some(self.sender.send_vote_cast(
            from,
            vote_plan,
            proposal_index,
            choice,
            &self.node,
        )?);
        Ok(self)
    }

    pub fn and_verify_is_rejected(
        self,
        duration: std::time::Duration,
    ) -> Result<Self, FragmentChainSenderError> {
        FragmentVerifier::wait_and_verify_is_rejected(
            duration,
            self.get_last_mempool_check()?,
            &self.node,
        )?;
        Ok(self)
    }

    pub fn and_verify_is_rejected_with_message(
        self,
        duration: std::time::Duration,
        message: impl Into<String>,
    ) -> Result<Self, FragmentChainSenderError> {
        FragmentVerifier::wait_and_verify_is_rejected_with_message(
            duration,
            self.get_last_mempool_check()?,
            message,
            &self.node,
        )?;
        Ok(self)
    }

    pub fn update_wallet(self, wallet: &mut Wallet, f: &dyn Fn(&mut Wallet)) -> Self {
        f(wallet);
        self
    }

    pub fn tally_vote(
        mut self,
        from: &mut Wallet,
        vote_plan: &VotePlan,
        tally_type: VoteTallyPayload,
    ) -> Result<Self, FragmentChainSenderError> {
        self.last_mempool_check = Some(
            self.sender
                .send_vote_tally(from, vote_plan, &self.node, tally_type)?,
        );
        Ok(self)
    }
}

#[derive(custom_debug::Debug, thiserror::Error)]
pub enum FragmentChainSenderError {
    #[error("fragment sender error")]
    FragmentSenderError(#[from] FragmentSenderError),
    #[error("fragment sender error")]
    FragmentVerifierError(#[from] FragmentVerifierError),
    #[error(
        "no fragment to verify. please send fragment first before calling any `and_verify*` method"
    )]
    NoFragmentToVerify,
}
