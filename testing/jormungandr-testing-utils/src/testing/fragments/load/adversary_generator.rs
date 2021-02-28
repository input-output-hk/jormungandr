use crate::testing::AdversaryFragmentSender;
use crate::testing::FragmentSender;
use crate::testing::FragmentSenderSetup;
use crate::testing::RemoteJormungandr;
use crate::testing::SyncNode;
use crate::wallet::Wallet;
use chain_impl_mockchain::fragment::FragmentId;
use jortestkit::load::{Id, RequestFailure, RequestGenerator};
use rand_core::OsRng;

pub struct AdversaryFragmentGenerator<'a, S: SyncNode + Send> {
    wallets: Vec<Wallet>,
    jormungandr: RemoteJormungandr,
    fragment_sender: FragmentSender<'a, S>,
    adversary_fragment_sender: AdversaryFragmentSender<'a, S>,
    rand: OsRng,
    split_marker: usize,
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

    pub fn send_transaction(&mut self) -> Result<FragmentId, RequestFailure> {
        self.increment_split_marker();
        let (senders, recievers) = self.wallets.split_at_mut(self.split_marker);
        let mut sender = senders.get_mut(senders.len() - 1).unwrap();
        let reciever = recievers.get(0).unwrap();

        self.adversary_fragment_sender
            .send_random_faulty_transaction(&mut sender, &reciever, &self.jormungandr)
            .map(|x| *x.fragment_id())
            .map_err(|e| RequestFailure::General(format!("{:?}", e)))
    }
}

impl<'a, S: SyncNode + Send> RequestGenerator for AdversaryFragmentGenerator<'a, S> {
    fn next(&mut self) -> Result<Vec<Option<Id>>, RequestFailure> {
        self.send_transaction()
            .map(|fragment_id| vec![Some(fragment_id.to_string())])
    }
}
