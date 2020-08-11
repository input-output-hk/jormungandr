use super::multi_controller::MultiControllerError;
use crate::{MultiController, Proposal};
use chain_impl_mockchain::fragment::FragmentId;
use jortestkit::load::{Id, RequestFailure, RequestGenerator};
use rand::RngCore;
use rand_core::OsRng;
use wallet_core::Choice;

pub struct WalletRequestGen {
    rand: OsRng,
    multi_controller: MultiController,
    initial_requests: Vec<Vec<u8>>,
    proposals: Vec<Proposal>,
}

impl WalletRequestGen {
    pub fn new(multi_controller: MultiController) -> Self {
        Self {
            multi_controller,
            initial_requests: Vec::new(),
            rand: OsRng,
            proposals: Vec::new(),
        }
    }

    pub fn fill_generator(&mut self) -> Result<(), MultiControllerError> {
        self.initial_requests = self.multi_controller.retrieve_conversion_transactions()?;
        self.proposals = self.multi_controller.proposals()?;
        Ok(())
    }

    pub fn next_usize(&mut self) -> usize {
        self.rand.next_u32() as usize
    }

    pub fn random_vote(&mut self) -> Result<FragmentId, MultiControllerError> {
        let proposal_index = self.next_usize() % self.proposals.len();
        let wallet_index = self.next_usize() % self.multi_controller.wallet_count();

        let proposal: Proposal = self.proposals.get(proposal_index).unwrap().clone();

        let options: Vec<u8> = proposal.chain_vote_options.0.values().cloned().collect();
        let choice_index = self.next_usize() % options.len();
        let choice = Choice::new(*options.get(choice_index).unwrap());

        self.multi_controller.refresh_wallet(wallet_index)?;
        self.multi_controller.vote(wallet_index, &proposal, choice)
    }

    pub fn send_conversion_fragment(&mut self, tx: Vec<u8>) -> Result<Option<Id>, RequestFailure> {
        let id = self
            .multi_controller
            .backend()
            .send_fragment(tx)
            .map_err(|e| RequestFailure::General(format!("{:?}", e)))?;
        self.multi_controller.confirm_transaction(id);
        Ok(Some(id.to_string()))
    }
}

impl RequestGenerator for WalletRequestGen {
    fn next(&mut self) -> Result<Option<Id>, RequestFailure> {
        if let Some(tx) = self.initial_requests.pop() {
            println!("Initial fragment_send");
            return self.send_conversion_fragment(tx);
        }
        let id = self
            .random_vote()
            .map_err(|e| RequestFailure::General(format!("{:?}", e)))?;
        Ok(Some(id.to_string()))
    }
}
