use crate::non_functional::voting::{
    config::{adversary_noise_config, PublicVotingLoadTestConfig},
    public::{adversary_public_vote_load_scenario, public_vote_load_scenario},
};

#[test]
pub fn public_vote_load_long_test() {
    let long_config = PublicVotingLoadTestConfig::long();
    public_vote_load_scenario(long_config)
}

#[test]
pub fn adversary_public_vote_load_long_test() {
    let long_config = PublicVotingLoadTestConfig::long();
    let adversary_noise_config = adversary_noise_config(30, long_config.test_duration());
    adversary_public_vote_load_scenario(long_config, adversary_noise_config)
}
