use crate::testing::FragmentSender;
use crate::testing::FragmentSenderSetup;
use crate::testing::RemoteJormungandr;
use crate::testing::SyncNode;
use crate::wallet::LinearFee;
use crate::wallet::Wallet;
use chain_impl_mockchain::fragment::Fragment;
use jormungandr_lib::crypto::hash::Hash;
use jortestkit::load::{Id, RequestFailure, RequestGenerator};
use rand_core::OsRng;

pub struct BatchFragmentGenerator<'a, S: SyncNode + Send> {
    wallets: Vec<Wallet>,
    jormungandr: RemoteJormungandr,
    fragment_sender: FragmentSender<'a, S>,
    rand: OsRng,
    split_marker: usize,
    batch_size: u8,
}

impl<'a, S: SyncNode + Send> BatchFragmentGenerator<'a, S> {
    pub fn new(
        fragment_sender_setup: FragmentSenderSetup<'a, S>,
        jormungandr: RemoteJormungandr,
        block_hash: Hash,
        fees: LinearFee,
        batch_size: u8,
    ) -> Self {
        Self {
            wallets: Vec::new(),
            fragment_sender: FragmentSender::new(block_hash, fees, fragment_sender_setup),
            rand: OsRng,
            jormungandr,
            split_marker: 0,
            batch_size,
        }
    }

    pub fn fill_from_faucet(&mut self, faucet: &mut Wallet) {
        let mut wallets: Vec<Wallet> =
            std::iter::from_fn(|| Some(Wallet::new_account(&mut self.rand)))
                .take(90)
                .collect();

        let fragment_sender = self
            .fragment_sender
            .clone_with_setup(FragmentSenderSetup::resend_3_times());
        fragment_sender
            .send_transaction_to_many(faucet, &wallets, &self.jormungandr, 1_000_000.into())
            .unwrap();

        let mut additional_wallets = Vec::new();

        for mut wallet in wallets.iter_mut().take(10) {
            let mut pack_of_wallets: Vec<Wallet> =
                std::iter::from_fn(|| Some(Wallet::new_account(&mut self.rand)))
                    .take(90)
                    .collect();
            fragment_sender
                .send_transaction_to_many(
                    &mut wallet,
                    &pack_of_wallets,
                    &self.jormungandr,
                    1000.into(),
                )
                .unwrap();
            additional_wallets.append(&mut pack_of_wallets);
        }
        self.wallets.append(&mut additional_wallets);
        self.wallets.append(&mut wallets);
    }

    pub fn increment_split_marker(&mut self) {
        self.split_marker += 1;
        if self.split_marker >= self.wallets.len() - 1 {
            self.split_marker = 1;
        }
    }

    pub fn generate_transaction(&mut self) -> Result<Fragment, RequestFailure> {
        self.increment_split_marker();
        let (senders, recievers) = self.wallets.split_at_mut(self.split_marker);
        let sender = senders.get_mut(senders.len() - 1).unwrap();
        let reciever = recievers.get(0).unwrap();

        let fragment = sender
            .transaction_to(
                &self.fragment_sender.block0_hash(),
                &self.fragment_sender.fees(),
                reciever.address(),
                1.into(),
            )
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

    pub fn send_batch(&mut self) -> Result<Vec<Option<Id>>, RequestFailure> {
        let transactions = self.generate_batch_transaction()?;
        self.fragment_sender
            .send_batch_fragments(transactions, false, &self.jormungandr)
            .map(|checks| {
                checks
                    .iter()
                    .map(|x| Some(x.fragment_id().to_string()))
                    .collect()
            })
            .map_err(|e| RequestFailure::General(format!("{:?}", e)))
    }
}

impl<S: SyncNode + Send> RequestGenerator for BatchFragmentGenerator<'_, S> {
    fn next(&mut self) -> Result<Vec<Option<Id>>, RequestFailure> {
        self.send_batch()
    }
}
