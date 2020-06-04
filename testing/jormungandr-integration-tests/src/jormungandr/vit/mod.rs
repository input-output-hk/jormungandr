use crate::common::{jcli_wrapper, jormungandr::{ConfigurationBuilder, Starter}, startup};
use assert_fs::TempDir;
use chain_impl_mockchain::vote::CommitteeId;

#[test]
pub fn test_jormungandr_get_committee_id() {
    let temp_dir = TempDir::new().unwrap();

    let expected_committee_ids = vec![
        CommitteeId::from_hex("7ef044ba437057d6d944ace679b7f811335639a689064cd969dffc8b55a7cc19").unwrap().into(),
        CommitteeId::from_hex("f5285eeead8b5885a1420800de14b0d1960db1a990a6c2f7b517125bedc000db").unwrap().into()
    ];

    let config = ConfigurationBuilder::new().with_committee_ids(expected_committee_ids.clone()).build(&temp_dir);
        
    let jormungandr = Starter::new()
        .config(config.clone())
        .start()
        .unwrap();
 
    startup::sleep_till_next_epoch(10, config.block0_configuration());

    let actual_committee_ids = jcli_wrapper::assert_get_active_voting_committees(&jormungandr.rest_uri());

    assert_eq!(expected_committee_ids,actual_committee_ids);
}