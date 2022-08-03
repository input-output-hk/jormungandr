use chain_crypto::bech32::Bech32;
use chain_impl_mockchain::{block::BlockDate, fee::LinearFee};
use jormungandr_automation::{jcli::JCli, jormungandr::ConfigurationBuilder};
use jormungandr_lib::interfaces::{ActiveSlotCoefficient, Mempool};
use tempfile::TempDir;

use crate::startup;

#[test]
pub fn fragment_id_bug() {
    let jcli: JCli = Default::default();
    let receiver = thor::Wallet::default();
    let sender = thor::Wallet::default();
    let fee = LinearFee::new(1, 1, 1);
    let value_to_send = 1;

    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        &[receiver.clone()],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(20)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(3)
            .with_linear_fees(fee)
            .with_mempool(Mempool {
                pool_max_entries: 1_000_000usize.into(),
                log_max_entries: 1_000_000usize.into(),
                persistent_log: None,
            }),
    )
    .unwrap();

    let dir = TempDir::new().unwrap();
    let sk_file = dir.path().join("sk");
    let staging = dir.path().join("staging");
    let sk = sender.secret_key().to_bech32_str();
    std::fs::write(&sk_file, sk).unwrap();

    let fragment_ids = jcli.transaction().make_transaction(
        jormungandr.rest_uri(),
        sender.address(),
        Some(receiver.address()),
        value_to_send.into(),
        jormungandr.genesis_block_hash().to_string(),
        BlockDate::first().next_epoch().into(),
        sk_file,
        staging,
        true,
    );

    assert_eq!(fragment_ids.len(), 1);

    let read_fragment = jcli.rest().v0().message().logs(jormungandr.rest_uri());
    assert_eq!(read_fragment.len(), 1);

    assert_eq!(fragment_ids[0], *read_fragment[0].fragment_id());
}
