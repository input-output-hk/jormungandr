use chain_impl_mockchain::fragment::FragmentId;
use jormungandr_automation::{jormungandr::RemoteJormungandr, testing::SyncNode};
use jortestkit::load::{Request, RequestFailure, RequestGenerator};
use loki::AdversaryFragmentSender;
use rand_core::OsRng;
use std::time::Instant;
use thor::{FragmentSender, FragmentSenderSetup, Wallet};

const DEFAULT_MAX_SPLITS: usize = 7; // equals to 128 splits, will likely not reach that value but it's there just to prevent a stack overflow

pub struct AdversaryFragmentGenerator<'a, S: SyncNode + Send> {
    wallets: Vec<Wallet>,
    jormungandr: RemoteJormungandr,
    fragment_sender: FragmentSender<'a, S>,
    adversary_fragment_sender: AdversaryFragmentSender<'a, S>,
    rand: OsRng,
    split_marker: usize,
    max_splits: usize, // avoid infinite splitting
}

impl<'a, S: SyncNode + Send> AdversaryFragmentGenerator<'a, S> {
    pub fn new(
        jormungandr: RemoteJormungandr,
        fragment_sender: FragmentSender<'a, S>,
        adversary_fragment_sender: AdversaryFragmentSender<'a, S>,
    ) -> Self {
        Self {
            wallets: Vec::new(),
            fragment_sender,
            adversary_fragment_sender,
            rand: OsRng,
            jormungandr,
            split_marker: 0,
            max_splits: DEFAULT_MAX_SPLITS,
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

    pub fn increment_split_marker(&mut self) {
        self.split_marker += 1;
        if self.split_marker >= self.wallets.len() - 1 {
            self.split_marker = 1;
        }
    }

    pub fn send_transaction(&mut self) -> Result<FragmentId, RequestFailure> {
        self.increment_split_marker();
        let (senders, recievers) = self.wallets.split_at_mut(self.split_marker);
        let sender = senders.get_mut(senders.len() - 1).unwrap();
        let reciever = recievers.get(0).unwrap();

        self.adversary_fragment_sender
            .send_random_faulty_transaction(sender, reciever, &self.jormungandr)
            .map(|x| *x.fragment_id())
            .map_err(|e| RequestFailure::General(format!("{:?}", e)))
    }
}

impl<'a, S: SyncNode + Send + Sync + Clone> RequestGenerator for AdversaryFragmentGenerator<'a, S> {
    fn next(&mut self) -> Result<Request, RequestFailure> {
        let start = Instant::now();
        self.send_transaction().map(|fragment_id| Request {
            ids: vec![Some(fragment_id.to_string())],
            duration: start.elapsed(),
        })
    }

    fn split(mut self) -> (Self, Option<Self>) {
        // Since no transaction will ever be accepted we could split as many times as we want
        // but that may trigger a bug in rayon so we artificially limit it
        if self.max_splits == 0 {
            return (self, None);
        }

        self.max_splits -= 1;

        let other = Self {
            wallets: self.wallets.clone(),
            jormungandr: self.jormungandr.clone_with_rest(),
            fragment_sender: self.fragment_sender.clone(),
            adversary_fragment_sender: self.adversary_fragment_sender.clone(),
            rand: OsRng,
            split_marker: 1,
            max_splits: self.max_splits,
        };
        (self, Some(other))
    }
}
