use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use chain_impl_mockchain::block::BlockDate;
use jormungandr_lib::interfaces::{Mempool, PersistentLog};
use jormungandr_testing_utils::testing::fragments::PersistentLogViewer;
use jormungandr_testing_utils::testing::jcli::JCli;
use jormungandr_testing_utils::testing::jormungandr::ConfigurationBuilder;
use jormungandr_testing_utils::testing::startup;
use jormungandr_testing_utils::testing::transaction_utils::TransactionHash;
pub use jortestkit::console::progress_bar::{parse_progress_bar_mode_from_str, ProgressBarMode};

#[test]
/// Verifies that no log entries are created for fragments that are already expired when received.
fn rejected_fragments_have_no_log() {
    let receiver = startup::create_new_account_address();
    let mut sender = startup::create_new_account_address();

    let log_path = TempDir::new().unwrap().child("log_path");

    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        &[receiver.clone()],
        ConfigurationBuilder::new()
            .with_slot_duration(1)
            .with_mempool(Mempool {
                pool_max_entries: 1_000.into(),
                log_max_entries: 1_000.into(),
                persistent_log: Some(PersistentLog {
                    dir: log_path.path().to_path_buf(),
                }),
            }),
    )
    .unwrap();

    let jcli = JCli::default();

    // Should be rejected without a log entry
    jcli.fragment_sender(&jormungandr)
        .send(
            &sender
                .transaction_to(
                    &jormungandr.genesis_block_hash(),
                    &jormungandr.fees(),
                    BlockDate::first(),
                    receiver.address(),
                    100.into(),
                )
                .unwrap()
                .encode(),
        )
        .assert_rejected_summary();

    // Should be accepted with a log entry
    jcli.fragment_sender(&jormungandr)
        .send(
            &sender
                .transaction_to(
                    &jormungandr.genesis_block_hash(),
                    &jormungandr.fees(),
                    BlockDate::first().next_epoch(),
                    receiver.address(),
                    101.into(),
                )
                .unwrap()
                .encode(),
        )
        .assert_in_block();

    // Should be rejected without a log entry
    jcli.fragment_sender(&jormungandr)
        .send(
            &sender
                .transaction_to(
                    &jormungandr.genesis_block_hash(),
                    &jormungandr.fees(),
                    BlockDate::first(),
                    receiver.address(),
                    102.into(),
                )
                .unwrap()
                .encode(),
        )
        .assert_rejected_summary();

    assert_eq!(
        PersistentLogViewer::new(log_path.path().to_path_buf()).count(),
        1
    );
}
