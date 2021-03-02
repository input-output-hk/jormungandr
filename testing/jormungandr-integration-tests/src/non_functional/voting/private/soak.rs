use crate::non_functional::voting::{
    config::{adversary_noise_config, PrivateVotingLoadTestConfig},
    private::{adversary_private_vote_load_scenario, private_vote_load_scenario},
};

#[test]
pub fn private_vote_load_long_test() {
    let quick_config = PrivateVotingLoadTestConfig::long();
    private_vote_load_scenario(quick_config)
}

#[test]
pub fn adversary_private_vote_load_long_test() {
    let long_config = PrivateVotingLoadTestConfig::long();
    let adversary_noise_config = adversary_noise_config(30, long_config.test_duration());
    adversary_private_vote_load_scenario(long_config, adversary_noise_config)
}
