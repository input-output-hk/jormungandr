use crate::common::{jormungandr::ConfigurationBuilder, startup};
use jormungandr_lib::interfaces::{ActiveSlotCoefficient, KesUpdateSpeed};
use jormungandr_testing_utils::testing::node::RestRequestGen;
use jortestkit::load::{self, Configuration, Monitor};

#[test]
pub fn rest_load_quick() {
    let faucet = startup::create_new_account_address();

    let (mut jormungandr, _) = startup::start_stake_pool(
        &[faucet],
        &[],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(4)
            .with_epoch_stability_depth(10)
            .with_kes_update_speed(KesUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();

    jormungandr.steal_temp_dir().unwrap().into_persistent();

    let rest_client = jormungandr.rest();
    let request = RestRequestGen::new(rest_client);
    let config = Configuration::duration(
        10,
        std::time::Duration::from_secs(40),
        10,
        Monitor::Progress(100),
        0,
        1_000,
    );
    let stats = load::start_sync(request, config, "Jormungandr rest load test");
    assert!((stats.calculate_passrate() as u32) > 95);
}
