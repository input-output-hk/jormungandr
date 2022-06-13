use crate::non_functional::voting::{
    config::adversary_noise_config,
    public::{adversary_public_vote_load_scenario, PublicVotingLoadTestConfig},
};

#[test]
pub fn adversary_public_vote_quick_test() {
    let quick_config = PublicVotingLoadTestConfig::quick();
    let adversary_noise_config = adversary_noise_config(30, quick_config.test_duration());
    adversary_public_vote_load_scenario(quick_config, adversary_noise_config)
}
