use crate::startup;
use chain_impl_mockchain::{block::BlockDate, testing::TestGen};
use jormungandr_automation::jormungandr::ConfigurationBuilder;

#[test]
pub fn evm_transaction_happy_path() {
    let mut alice = thor::Wallet::default();
    let bob = thor::Wallet::default();

    let (jormungandr, _stake_pools) =
        startup::start_stake_pool(&[alice.clone()], &[bob], &mut ConfigurationBuilder::new())
            .unwrap();

    let transaction_sender = thor::FragmentSender::from(jormungandr.block0_configuration());

    let fragment_builder = thor::FragmentBuilder::new(
        &jormungandr.genesis_block_hash(),
        &jormungandr.fees(),
        BlockDate::first().next_epoch(),
    );

    let evm_mapping = TestGen::evm_mapping_for_wallet(&alice.clone().into());

    assert_eq!(
        "",
        jormungandr
            .rest()
            .raw()
            .evm_address(evm_mapping.account_id())
            .unwrap()
            .text()
            .unwrap(),
        "Evm address already existing"
    );

    let alice_fragment = fragment_builder.evm_mapping(&alice, &evm_mapping);

    transaction_sender
        .send_fragment(&mut alice, alice_fragment, &jormungandr)
        .unwrap();

    assert_eq!(
        evm_mapping.evm_address().to_string(),
        jormungandr
            .rest()
            .evm_address(evm_mapping.account_id())
            .unwrap(),
        "Evm address not equal"
    );

    assert_eq!(
        evm_mapping.account_id().to_string(),
        jormungandr
            .rest()
            .jor_address(evm_mapping.evm_address())
            .unwrap(),
        "Jor address not equal"
    );
}