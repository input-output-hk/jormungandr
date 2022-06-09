use super::wallet_lane_iter::SplitLaneIter;
use chain_impl_mockchain::{fee::LinearFee, fragment::Fragment};
use jormungandr_automation::{jormungandr::RemoteJormungandr, testing::SyncNode};
use jormungandr_lib::crypto::hash::Hash;
use jortestkit::load::{Request, RequestFailure, RequestGenerator};
use rand_core::OsRng;
use std::time::Instant;
use thor::{BlockDateGenerator, FragmentBuilder, FragmentSender, FragmentSenderSetup, Wallet};

pub struct BatchFragmentGenerator<'a, S: SyncNode + Send> {
    wallets: Vec<Wallet>,
    jormungandr: RemoteJormungandr,
    fragment_sender: FragmentSender<'a, S>,
    rand: OsRng,
    split_lane: SplitLaneIter,
    batch_size: u8,
}

impl<'a, S: SyncNode + Send> BatchFragmentGenerator<'a, S> {
    pub fn new(
        fragment_sender_setup: FragmentSenderSetup<'a, S>,
        jormungandr: RemoteJormungandr,
        block_hash: Hash,
        fees: LinearFee,
        expiry_generator: BlockDateGenerator,
        batch_size: u8,
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
            batch_size,
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

    pub fn generate_transaction(&mut self) -> Result<Fragment, RequestFailure> {
        let (split_marker, witness_mode) = self.split_lane.next(self.wallets.len());
        let (senders, recievers) = self.wallets.split_at_mut(split_marker);
        let sender = senders.get_mut(senders.len() - 1).unwrap();
        let reciever = recievers.get(0).unwrap();

        let fragment = FragmentBuilder::new(
            &self.fragment_sender.block0_hash(),
            &self.fragment_sender.fees(),
            self.fragment_sender.date(),
        )
        .witness_mode(witness_mode)
        .transaction(sender, reciever.address(), 1.into())
        .map_err(|e| RequestFailure::General(format!("{:?}", e)));
        sender.confirm_transaction();
        fragment
    }

    pub fn batch_size(&self) -> u8 {
        self.batch_size
    }

    pub fn generate_batch_transaction(&mut self) -> Result<Vec<Fragment>, RequestFailure> {
        let mut transactions = vec![];

        for _ in 0..self.batch_size {
            transactions.push(self.generate_transaction()?);
        }
        Ok(transactions)
    }

    pub fn send_batch(&mut self) -> Result<Request, RequestFailure> {
        let transactions = self.generate_batch_transaction()?;
        let start = Instant::now();
        self.fragment_sender
            .send_batch_fragments(transactions, false, &self.jormungandr)
            .map(|summary| Request {
                ids: summary
                    .fragment_ids()
                    .iter()
                    .map(|x| Some(x.to_string()))
                    .collect(),
                duration: start.elapsed(),
            })
            .map_err(|e| RequestFailure::General(format!("{:?}", e)))
    }
}

impl<S: SyncNode + Send + Sync + Clone> RequestGenerator for BatchFragmentGenerator<'_, S> {
    fn next(&mut self) -> Result<Request, RequestFailure> {
        self.send_batch()
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
            batch_size: self.batch_size(),
        };
        (self, Some(other))
    }
}
