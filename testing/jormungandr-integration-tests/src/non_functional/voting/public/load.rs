use crate::non_functional::voting::{
    config::PublicVotingLoadTestConfig, public::public_vote_load_scenario,
};

#[test]
pub fn public_vote_load_quick_test() {
    let quick_config = PublicVotingLoadTestConfig::quick();
    public_vote_load_scenario(quick_config)
}
