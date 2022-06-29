use crate::non_functional::voting::{
    config::adversary_noise_config,
    private::{adversary_private_vote_load_scenario, PrivateVotingLoadTestConfig},
};

#[test]
pub fn adversary_private_vote_quick_test() {
    let quick_config = PrivateVotingLoadTestConfig::quick();
    let adversary_noise_config = adversary_noise_config(30, quick_config.test_duration());
    adversary_private_vote_load_scenario(quick_config, adversary_noise_config)
}
