use crate::common::{
    jcli_wrapper,
    jormungandr::{ConfigurationBuilder, Starter},
};
use assert_fs::TempDir;
use chain_impl_mockchain::{testing::VoteTestGen, vote::CommitteeId};
use jormungandr_lib::{crypto::key::KeyPair, interfaces::CommitteeIdDef};
use jormungandr_testing_utils::wallet::Wallet;
use rand::rngs::OsRng;
use rand_core::{CryptoRng, RngCore};

const TEST_COMMITTEE_SIZE: usize = 3;

fn generate_wallets_and_committee<RNG>(rng: &mut RNG) -> (Vec<Wallet>, Vec<CommitteeIdDef>)
where
    RNG: CryptoRng + RngCore,
{
    let mut ids = Vec::new();
    let mut wallets = Vec::new();
    for _i in 0..TEST_COMMITTEE_SIZE {
        let wallet = Wallet::new_account(rng);
        let id = CommitteeIdDef::from(CommitteeId::from(
            wallet.address().1.public_key().unwrap().clone(),
        ));
        ids.push(id);
        wallets.push(wallet);
    }
    (wallets, ids)
}

#[test]
pub fn test_get_committee_id() {
    let temp_dir = TempDir::new().unwrap();

    let mut rng = OsRng;
    let (_, mut expected_committee_ids) = generate_wallets_and_committee(&mut rng);

    let leader_key_pair = KeyPair::generate(&mut rng);

    let config = ConfigurationBuilder::new()
        .with_leader_key_pair(leader_key_pair.clone())
        .with_committee_ids(expected_committee_ids.clone())
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

    expected_committee_ids.insert(
        0,
        CommitteeIdDef::from(CommitteeId::from(
            leader_key_pair.identifier().into_public_key(),
        )),
    );

    let actual_committee_ids =
        jcli_wrapper::assert_get_active_voting_committees(&jormungandr.rest_uri());

    assert_eq!(expected_committee_ids, actual_committee_ids);
}

#[test]
pub fn test_get_initial_vote_plan() {
    let temp_dir = TempDir::new().unwrap();

    let mut rng = OsRng;
    let (wallets, expected_committee_ids) = generate_wallets_and_committee(&mut rng);

    let expected_vote_plan = VoteTestGen::vote_plan();

    let vote_plan_cert =
        jormungandr_testing_utils::testing::vote_plan_cert(&wallets[0], &expected_vote_plan).into();

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
