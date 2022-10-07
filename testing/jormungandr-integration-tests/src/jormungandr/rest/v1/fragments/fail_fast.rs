use crate::startup;
use chain_impl_mockchain::{block::BlockDate, fragment::Fragment};
use jormungandr_automation::jormungandr::{
    assert_bad_request, Block0ConfigurationBuilder, JormungandrProcess, MemPoolCheck,
    NodeConfigBuilder,
};
use loki::FaultyTransactionBuilder;
use rstest::*;
use std::time::Duration;
use thor::{FragmentSender, FragmentVerifier};

#[fixture]
fn world() -> (
    JormungandrProcess,
    Fragment,
    Fragment,
    Fragment,
    Fragment,
    Fragment,
) {
    let alice = thor::Wallet::default();
    let bob = thor::Wallet::default();
    let clarice = thor::Wallet::default();
    let david = thor::Wallet::default();

    let (jormungandr, _stake_pools) = startup::start_stake_pool(
        &[alice.clone()],
        &[bob.clone(), clarice.clone()],
        Block0ConfigurationBuilder::default(),
        NodeConfigBuilder::default(),
    )
    .unwrap();

    let settings = jormungandr.rest().settings().unwrap();

    let fragment_builder =
        thor::FragmentBuilder::from_settings(&settings, BlockDate::first().next_epoch());

    let alice_fragment = fragment_builder
        .transaction(&alice, bob.address(), 100.into())
        .unwrap();

    let bob_fragment = fragment_builder
        .transaction(&bob, alice.address(), 100.into())
        .unwrap();
    let clarice_fragment = fragment_builder
        .transaction(&clarice, alice.address(), 100.into())
        .unwrap();

    let late_invalid_fragment = fragment_builder
        .transaction(&david, alice.address(), 100.into())
        .unwrap();

    let faulty_tx_builder = FaultyTransactionBuilder::from_settings(
        jormungandr.rest().settings().unwrap(),
        BlockDate::first().next_epoch(),
    );
    let early_invalid_fragment = faulty_tx_builder.unbalanced(&alice, &bob);

    (
        jormungandr,
        alice_fragment,
        bob_fragment,
        clarice_fragment,
        early_invalid_fragment,
        late_invalid_fragment,
    )
}

#[rstest]
pub fn fail_fast_on_all_valid(
    world: (
        JormungandrProcess,
        Fragment,
        Fragment,
        Fragment,
        Fragment,
        Fragment,
    ),
) {
    let (jormungandr, valid_fragment_1, valid_fragment_2, valid_fragment_3, _, _) = world;
    let transaction_sender = FragmentSender::from(&jormungandr.rest().settings().unwrap());
    let tx_ids: Vec<MemPoolCheck> = transaction_sender
        .send_batch_fragments(
            vec![valid_fragment_1, valid_fragment_2, valid_fragment_3],
            true,
            &jormungandr,
        )
        .unwrap()
        .fragment_ids()
        .into_iter()
        .map(MemPoolCheck::from)
        .collect();

    FragmentVerifier::wait_for_all_fragments(Duration::from_secs(5), &jormungandr).unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_all_valid(&tx_ids);
}

#[rstest]
pub fn fail_fast_off_all_valid(
    world: (
        JormungandrProcess,
        Fragment,
        Fragment,
        Fragment,
        Fragment,
        Fragment,
    ),
) {
    let (jormungandr, valid_fragment_1, valid_fragment_2, valid_fragment_3, _, _) = world;
    let transaction_sender = FragmentSender::from(&jormungandr.rest().settings().unwrap());
    let tx_ids: Vec<MemPoolCheck> = transaction_sender
        .send_batch_fragments(
            vec![valid_fragment_1, valid_fragment_2, valid_fragment_3],
            false,
            &jormungandr,
        )
        .unwrap()
        .fragment_ids()
        .into_iter()
        .map(MemPoolCheck::from)
        .collect();

    FragmentVerifier::wait_for_all_fragments(Duration::from_secs(5), &jormungandr).unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_all_valid(&tx_ids);
}

#[rstest]
pub fn fail_fast_on_first_invalid(
    world: (
        JormungandrProcess,
        Fragment,
        Fragment,
        Fragment,
        Fragment,
        Fragment,
    ),
) {
    let (jormungandr, valid_fragment_1, valid_fragment_2, _, early_invalid_fragment, _) = world;
    assert_bad_request(jormungandr.rest().send_fragment_batch(
        vec![early_invalid_fragment, valid_fragment_1, valid_fragment_2],
        true,
    ));

    FragmentVerifier::wait_for_all_fragments(Duration::from_secs(5), &jormungandr).unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_no_fragments();
}

#[rstest]
pub fn fail_fast_on_first_late_invalid(
    world: (
        JormungandrProcess,
        Fragment,
        Fragment,
        Fragment,
        Fragment,
        Fragment,
    ),
) {
    let (jormungandr, valid_fragment_1, valid_fragment_2, _, _, late_invalid_fragment) = world;

    let fragments = vec![late_invalid_fragment, valid_fragment_1, valid_fragment_2];

    FragmentSender::from(&jormungandr.rest().settings().unwrap())
        .send_batch_fragments(fragments.clone(), true, &jormungandr)
        .unwrap();

    FragmentVerifier::wait_for_all_fragments(Duration::from_secs(5), &jormungandr).unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_invalid(&fragments[0].hash().into())
        .assert_valid(&fragments[1].hash().into())
        .assert_valid(&fragments[2].hash().into());
}

#[rstest]
pub fn fail_fast_off_first_invalid(
    world: (
        JormungandrProcess,
        Fragment,
        Fragment,
        Fragment,
        Fragment,
        Fragment,
    ),
) {
    let (jormungandr, valid_fragment_1, valid_fragment_2, _, early_invalid_fragment, _) = world;
    let tx_ids = assert_bad_request(jormungandr.rest().send_fragment_batch(
        vec![valid_fragment_1, valid_fragment_2, early_invalid_fragment],
        true,
    ));

    FragmentVerifier::wait_for_all_fragments(Duration::from_secs(5), &jormungandr).unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_not_exist(&tx_ids[2])
        .assert_valid(&tx_ids[0])
        .assert_valid(&tx_ids[1]);
}

#[rstest]
pub fn fail_fast_off_invalid_in_middle(
    world: (
        JormungandrProcess,
        Fragment,
        Fragment,
        Fragment,
        Fragment,
        Fragment,
    ),
) {
    let (jormungandr, valid_fragment_1, valid_fragment_2, _, early_invalid_fragment, _) = world;
    let tx_ids = assert_bad_request(jormungandr.rest().send_fragment_batch(
        vec![valid_fragment_1, early_invalid_fragment, valid_fragment_2],
        false,
    ));

    FragmentVerifier::wait_for_all_fragments(Duration::from_secs(5), &jormungandr).unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_valid(&tx_ids[0])
        .assert_valid(&tx_ids[2])
        .assert_not_exist(&tx_ids[1]);
}

#[rstest]
pub fn fail_fast_on_invalid_in_middle(
    world: (
        JormungandrProcess,
        Fragment,
        Fragment,
        Fragment,
        Fragment,
        Fragment,
    ),
) {
    let (jormungandr, valid_fragment_1, valid_fragment_2, _, early_invalid_fragment, _) = world;
    let tx_ids = assert_bad_request(jormungandr.rest().send_fragment_batch(
        vec![valid_fragment_1, early_invalid_fragment, valid_fragment_2],
        true,
    ));

    FragmentVerifier::wait_for_all_fragments(Duration::from_secs(5), &jormungandr).unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_valid(&tx_ids[0])
        .assert_not_exist(&tx_ids[1])
        .assert_not_exist(&tx_ids[2]);
}
#[rstest]
pub fn fail_fast_on_last_invalid(
    world: (
        JormungandrProcess,
        Fragment,
        Fragment,
        Fragment,
        Fragment,
        Fragment,
    ),
) {
    let (jormungandr, valid_fragment_1, valid_fragment_2, _, early_invalid_fragment, _) = world;
    let tx_ids = assert_bad_request(jormungandr.rest().send_fragment_batch(
        vec![valid_fragment_1, valid_fragment_2, early_invalid_fragment],
        true,
    ));

    FragmentVerifier::wait_for_all_fragments(Duration::from_secs(5), &jormungandr).unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_valid(&tx_ids[0])
        .assert_valid(&tx_ids[1])
        .assert_not_exist(&tx_ids[2]);
}

#[rstest]
pub fn fail_fast_off_last_invalid(
    world: (
        JormungandrProcess,
        Fragment,
        Fragment,
        Fragment,
        Fragment,
        Fragment,
    ),
) {
    let (jormungandr, valid_fragment_1, valid_fragment_2, _, early_invalid_fragment, _) = world;
    let tx_ids = assert_bad_request(jormungandr.rest().send_fragment_batch(
        vec![valid_fragment_1, valid_fragment_2, early_invalid_fragment],
        false,
    ));

    FragmentVerifier::wait_for_all_fragments(Duration::from_secs(5), &jormungandr).unwrap();

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_valid(&tx_ids[0])
        .assert_valid(&tx_ids[1])
        .assert_not_exist(&tx_ids[2]);
}
