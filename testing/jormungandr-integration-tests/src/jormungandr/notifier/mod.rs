use crate::common::{
    jcli_wrapper,
    jormungandr::{notifier::NotifierMessage, ConfigurationBuilder},
    process_utils::Wait,
    startup,
    transaction_utils::TransactionHash,
};
use jormungandr_lib::interfaces::{notifier::JsonMessage, ActiveSlotCoefficient};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

#[test]
pub fn notifier_shows_the_same_tip_as_rest() {
    let faucet = startup::create_new_account_address();

    let mut config = ConfigurationBuilder::new();
    config.with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM);

    let (jormungandr, _) = startup::start_stake_pool(&[faucet], &[], &mut config).unwrap();

    let rest = jormungandr.rest();

    let mut notifier = jormungandr.notifier();

    let mut counter = Arc::new(AtomicUsize::new(0));

    notifier
        .new_block_client(move |msg| {
            if let NotifierMessage::JsonMessage(JsonMessage::NewTip(hash)) = msg {
                let rest_tip = rest.tip().expect("couldn't get tip from rest");
                assert_eq!(hash, rest_tip);

                let count = Arc::get_mut(&mut counter).unwrap();

                let count = count.fetch_add(1, Ordering::AcqRel);

                if count == 5 {
                    return false;
                }
            };
            true
        })
        .expect("couldn't connect client");

    notifier.wait_all().unwrap();
}

#[test]
pub fn notifier_fails_with_more_than_max_connections() {
    let faucet = startup::create_new_account_address();

    let mut config = ConfigurationBuilder::new();
    config
        .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
        .with_notifier_max_connections(1);

    let (jormungandr, _) = startup::start_stake_pool(&[faucet], &[], &mut config).unwrap();

    let mut notifier = jormungandr.notifier();

    let waiting = Arc::new(AtomicBool::new(true));
    let waiting1 = Arc::clone(&waiting);

    notifier
        .new_block_client(move |_msg| waiting1.load(Ordering::Acquire))
        .expect("couldn't connect client");

    notifier
        .new_block_client(move |msg| {
            match msg {
                NotifierMessage::MaxConnectionsReached => {
                    waiting.store(false, Ordering::Release);
                }
                _ => unreachable!("shouldn't be able to connect"),
            }

            true
        })
        .expect("couldn't connect client");

    notifier.wait_all().unwrap();
}

#[test]
pub fn mempool_fragment_accepted() {
    let mut faucet = startup::create_new_account_address();
    let receiver = startup::create_new_account_address();

    let mut config = ConfigurationBuilder::new();
    config.with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM);

    let (jormungandr, _) = startup::start_stake_pool(&[faucet.clone()], &[], &mut config).unwrap();

    let transaction = faucet
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            receiver.address(),
            1_000.into(),
        )
        .unwrap();

    let transaction_id = transaction.hash();

    let mut notifier = jormungandr.notifier();

    {
        notifier
            .new_mempool_client(move |msg| {
                match msg {
                    NotifierMessage::JsonMessage(JsonMessage::FragmentAccepted(fragment_id)) => {
                        assert_eq!(fragment_id, transaction_id.into());
                    }
                    NotifierMessage::MaxConnectionsReached => {
                        unreachable!("shouldn't reach max connections")
                    }
                    _ => unreachable!("unexpected message"),
                }

                false
            })
            .expect("couldn't connect client");
    }

    let wait = Wait::new(Duration::from_secs(3), 20);

    let transaction = transaction.encode();
    let _fragment_id =
        jcli_wrapper::assert_transaction_in_block_with_wait(&transaction, &jormungandr, &wait);

    notifier.wait_all().unwrap();
}
