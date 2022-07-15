use super::wallet_lane_iter::SplitLaneIter;
use chain_impl_mockchain::{fee::LinearFee, fragment::FragmentId};
use jormungandr_automation::{jormungandr::RemoteJormungandr, testing::SyncNode};
use jormungandr_lib::crypto::hash::Hash;
use jortestkit::load::{Request, RequestFailure, RequestGenerator};
use rand_core::OsRng;
use std::time::Instant;
use thor::{BlockDateGenerator, FragmentSender, FragmentSenderSetup, Wallet};
pub struct TransactionGenerator<'a, S: SyncNode + Send> {
    wallets: Vec<Wallet>,
    jormungandr: RemoteJormungandr,
    fragment_sender: FragmentSender<'a, S>,
    rand: OsRng,
    split_lane: SplitLaneIter,
}

impl<'a, S: SyncNode + Send> TransactionGenerator<'a, S> {
    pub fn new(
        fragment_sender_setup: FragmentSenderSetup<'a, S>,
        jormungandr: RemoteJormungandr,
        block_hash: Hash,
        fees: LinearFee,
        expiry_generator: BlockDateGenerator,
    ) -> Self {
        Self {
            wallets: Vec::new(),
            fragment_sender: FragmentSender::new(
                block_hash,
                fees,
                expiry_generator,
                fragment_sender_setup,
            ),
            rand: OsRng,
            jormungandr,
            split_lane: SplitLaneIter::new(),
        }
    }

    pub fn fill_from_faucet(&mut self, faucet: &mut Wallet) {
        let discrimination = self.jormungandr.rest().settings().unwrap().discrimination;

        let mut wallets: Vec<Wallet> =
            std::iter::from_fn(|| Some(Wallet::new_account(&mut self.rand, discrimination)))
                .take(90)
                .collect();

        let fragment_sender = self
            .fragment_sender
            .clone_with_setup(FragmentSenderSetup::resend_3_times());
        fragment_sender
            .send_transaction_to_many(faucet, &wallets, &self.jormungandr, 1_000_000.into())
            .unwrap();

        let mut additional_wallets = Vec::new();

        for wallet in wallets.iter_mut().take(10) {
            let mut pack_of_wallets: Vec<Wallet> =
                std::iter::from_fn(|| Some(Wallet::new_account(&mut self.rand, discrimination)))
                    .take(90)
                    .collect();
            fragment_sender
                .send_transaction_to_many(wallet, &pack_of_wallets, &self.jormungandr, 1000.into())
                .unwrap();
            additional_wallets.append(&mut pack_of_wallets);
        }
        self.wallets.append(&mut additional_wallets);
        self.wallets.append(&mut wallets);
    }

    pub fn send_transaction(&mut self) -> Result<FragmentId, RequestFailure> {
        let (split_marker, witness_mode) = self.split_lane.next(self.wallets.len());
        let (senders, recievers) = self.wallets.split_at_mut(split_marker);
        let sender = senders.get_mut(senders.len() - 1).unwrap();
        let reciever = recievers.get(0).unwrap();

        self.fragment_sender
            .send_transaction_with_witness_mode(
                sender,
                reciever.address(),
                &self.jormungandr,
                1.into(),
                witness_mode,
            )
            .map(|x| *x.fragment_id())
            .map_err(|e| RequestFailure::General(format!("{:?}", e)))
    }
}

impl<S: SyncNode + Send + Sync + Clone> RequestGenerator for TransactionGenerator<'_, S> {
    fn next(&mut self) -> Result<Request, RequestFailure> {
        let start = Instant::now();
        self.send_transaction().map(|fragment_id| Request {
            ids: vec![Some(fragment_id.to_string())],
            duration: start.elapsed(),
        })
    }

    fn split(mut self) -> (Self, Option<Self>) {
        let wallets_len = self.wallets.len();
        // there needs to be at least one receiver and one sender in each half
        if wallets_len <= 4 {
            return (self, None);
        }
        let wallets = self.wallets.split_off(wallets_len / 2);
        let other = Self {
            wallets,
            jormungandr: self.jormungandr.clone_with_rest(),
            fragment_sender: self.fragment_sender.clone(),
            rand: OsRng,
            split_lane: SplitLaneIter::new(),
        };
        (self, Some(other))
    }
}
