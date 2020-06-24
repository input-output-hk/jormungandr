use crate::common::{
    jormungandr::{notifier::JsonMessage, ConfigurationBuilder},
    startup,
};

use jormungandr_lib::interfaces::ActiveSlotCoefficient;
use std::sync::{Arc, Condvar, Mutex};

#[test]
pub fn notifier_shows_the_same_tip_as_rest() {
    let faucet = startup::create_new_account_address();

    let mut config = ConfigurationBuilder::new();
    config.with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM);

    let (jormungandr, _) = startup::start_stake_pool(&[faucet], &[], &mut config).unwrap();

    let rest = jormungandr.rest();

    let mut notifier = jormungandr.notifier();

    #[allow(clippy::mutex_atomic)]
    let pair = Arc::new((Mutex::new(0usize), Condvar::new()));
    let pair2 = pair.clone();

    notifier
        .new_client(move |msg| {
            if let JsonMessage::NewTip(hash) = msg {
                let rest_tip = rest.tip().expect("couldn't get tip from rest");
                assert_eq!(hash, rest_tip);

                let (lock, cvar) = &*pair2;
                let mut done = lock.lock().unwrap();
                *done += 1;
                cvar.notify_one();
            };
        })
        .expect("couldn't connect client");

    let (lock, cvar) = &*pair;
    let mut done = lock.lock().unwrap();
    while !*done < 5 {
        done = cvar.wait(done).unwrap();
    }
}
