use crate::startup;
use jormungandr_automation::jormungandr::{
    explorer::{configuration::ExplorerParams, verifier::ExplorerVerifier},
    ConfigurationBuilder,
};
use jormungandr_lib::interfaces::ActiveSlotCoefficient;

#[test]
pub fn explorer_address_test() {
    let sender = thor::Wallet::default();
    let address_bech32_prefix = "ca".to_string();

    let mut config = ConfigurationBuilder::new();
    config.with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM);

    let (jormungandr, _initial_stake_pools) =
        startup::start_stake_pool(&[sender.clone()], &[], &mut config).unwrap();

    let params = ExplorerParams::new(None, None, address_bech32_prefix);
    let explorer_process = jormungandr.explorer(params);
    let explorer = explorer_process.client();

    let explorer_address = explorer.address(sender.address().to_string()).unwrap();

    assert!(
        explorer_address.errors.is_none(),
        "{:?}",
        explorer_address.errors.unwrap()
    );

    ExplorerVerifier::assert_address(sender.address(), explorer_address.data.unwrap().address);
}
