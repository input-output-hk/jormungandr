use crate::startup;
use jormungandr_automation::{jcli::JCli, jormungandr::ConfigurationBuilder};
use jormungandr_lib::interfaces::ActiveSlotCoefficient;
#[ignore]
#[test]
pub fn explorer_address_test() {
    let _jcli: JCli = Default::default();
    let sender = thor::Wallet::default();
    let _receiver = thor::Wallet::default();
    let _transaction_value = 1_000;

    let mut config = ConfigurationBuilder::new();
    config.with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM);

    let (jormungandr, _initial_stake_pools) =
        startup::start_stake_pool(&[sender.clone()], &[], &mut config).unwrap();

    let explorer_process = jormungandr.explorer();
    let explorer = explorer_process.client();

    explorer
        .address(sender.address_bech32(chain_addr::Discrimination::Test))
        .unwrap();
}
