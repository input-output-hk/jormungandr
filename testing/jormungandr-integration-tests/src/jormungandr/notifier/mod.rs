use crate::common::{
    jormungandr::{notifier::NotifierMessage, ConfigurationBuilder},
    startup,
};

use jormungandr_lib::interfaces::{notifier::JsonMessage, ActiveSlotCoefficient};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

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
        .new_client(move |msg| {
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
        .new_client(move |_msg| waiting1.load(Ordering::Acquire))
        .expect("couldn't connect client");

    notifier
        .new_client(move |msg| {
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
