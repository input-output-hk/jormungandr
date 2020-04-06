use crate::common::{
    jcli_wrapper, jormungandr::ConfigurationBuilder, process_utils::Wait, startup,
    transaction_utils::TransactionHash,
};

use jormungandr_lib::interfaces::ActiveSlotCoefficient;
use std::time::Duration;

#[test]
pub fn explorer_test() {
    let mut faucet = startup::create_new_account_address();
    let receiver = startup::create_new_account_address();

    let mut config = ConfigurationBuilder::new();
    config
        .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
        .with_explorer();

    let (jormungandr, _) = startup::start_stake_pool(&[faucet.clone()], &[], &mut config).unwrap();

    let transaction = faucet
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            receiver.address(),
            1_000.into(),
        )
        .unwrap()
        .encode();

    let wait = Wait::new(Duration::from_secs(3), 20);

    let fragment_id =
        jcli_wrapper::assert_transaction_in_block_with_wait(&transaction, &jormungandr, &wait);

    let explorer = jormungandr.explorer();
    let explorer_transaction = explorer
        .get_transaction(fragment_id)
        .expect("non existing transaction");
    assert_eq!(
        fragment_id, explorer_transaction.id,
        "incorrect fragment id"
    );
}
