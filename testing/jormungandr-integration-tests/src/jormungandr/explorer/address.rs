use crate::startup;
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{explorer::configuration::ExplorerParams, ConfigurationBuilder},
};
use jormungandr_lib::interfaces::ActiveSlotCoefficient;
//TODO still wip
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

    let params = ExplorerParams {
        address_bech32_prefix: "ca".to_string().into(),
        ..Default::default()
    };
    let explorer_process = jormungandr.explorer(params);
    let explorer = explorer_process.client();

    let explorer_address = explorer.address(sender.address().to_string()).unwrap();

    assert!(
        explorer_address.errors.is_none(),
        "{:?}",
        explorer_address.errors.unwrap()
    );

    assert_eq!(
        explorer_address.data.unwrap().address.id,
        sender.address().to_string(),
        "Addresses not the same"
    );
}
