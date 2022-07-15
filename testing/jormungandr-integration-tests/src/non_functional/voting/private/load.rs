use crate::non_functional::voting::private::{
    private_vote_load_scenario, PrivateVotingLoadTestConfig,
};

#[test]
pub fn private_vote_load_quick_test() {
    let quick_config = PrivateVotingLoadTestConfig::quick();
    private_vote_load_scenario(quick_config)
}
