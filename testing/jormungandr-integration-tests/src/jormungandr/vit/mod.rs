use crate::common::{
    jcli_wrapper,
    jormungandr::{ConfigurationBuilder, Starter},
};
use assert_fs::TempDir;
use chain_impl_mockchain::{testing::VoteTestGen, vote::CommitteeId};
use jormungandr_lib::interfaces::CommitteeIdDef;

fn committee_ids() -> Vec<CommitteeIdDef> {
    vec![
        CommitteeId::from_hex("7ef044ba437057d6d944ace679b7f811335639a689064cd969dffc8b55a7cc19")
            .unwrap()
            .into(),
        CommitteeId::from_hex("f5285eeead8b5885a1420800de14b0d1960db1a990a6c2f7b517125bedc000db")
            .unwrap()
            .into(),
    ]
}

#[test]
pub fn test_get_committee_id() {
    let temp_dir = TempDir::new().unwrap();

    let expected_committee_ids = committee_ids();

    let config = ConfigurationBuilder::new()
        .with_committee_ids(expected_committee_ids.clone())
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

    let actual_committee_ids =
        jcli_wrapper::assert_get_active_voting_committees(&jormungandr.rest_uri());

    assert_eq!(expected_committee_ids, actual_committee_ids);
}

#[test]
pub fn test_get_initial_vote_plan() {
    let temp_dir = TempDir::new().unwrap();

    let expected_committee_ids = committee_ids();

    let expected_vote_plan = VoteTestGen::vote_plan();
    let vote_plan_cert =
        jormungandr_testing_utils::testing::vote_plan_cert(&expected_vote_plan).into();

    let config = ConfigurationBuilder::new()
        .with_committee_ids(expected_committee_ids.clone())
        .with_certs(vec![vote_plan_cert])
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

    let vote_plans = jcli_wrapper::assert_get_active_vote_plans(&jormungandr.rest_uri());

    assert!(vote_plans.len() == 1);

    let vote_plan = vote_plans.get(0).unwrap();
    let actual_vote_plan_id = vote_plan["id"].as_str().unwrap().to_string();

    assert_eq!(actual_vote_plan_id, expected_vote_plan.to_id().to_string());
}
