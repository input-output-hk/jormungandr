use jortestkit::load::Configuration;
use jortestkit::load::Monitor;
use jortestkit::prelude::ResourcesUsage;
use std::time::Duration;

pub struct VotingLoadTestConfig {
    name: String,
    rewards_increase: u64,
    configuration: Configuration,
    initial_fund_per_wallet: u64,
    wallets_count: usize,
    slot_duration: u8,
    slots_in_epoch: u32,
    voting_timing: Vec<u32>,
    block_content_max_size: u32,
    proposals_count: usize,
    target_resources_usage: ResourcesUsage,
    tx_target_success_rate: u32,
}

impl VotingLoadTestConfig {
    pub fn total_votes(&self) -> u32 {
        self.configuration.total_votes()
    }

    pub fn test_duration(&self) -> std::time::Duration {
        let as_secs: u64 = (self.slot_duration as u64)
            * (self.slots_in_epoch as u64)
            * ((self.voting_timing[2] - self.voting_timing[1]) as u64);
        std::time::Duration::from_secs(as_secs)
    }

    pub fn measurement_name<S: Into<String>>(&self, prefix: S) -> String {
        format!(
            "{}_vote_test_with_{}_votes_and_{}_voters_{}",
            prefix.into(),
            self.total_votes(),
            self.wallets_count,
            self.name
        )
    }

    pub fn quick() -> Self {
        Self {
            name: "quick".to_string(),
            rewards_increase: 10u64,
            configuration: Configuration::requests_per_thread(
                5,
                250,
                100,
                Monitor::Standard(100),
                100,
            ),
            initial_fund_per_wallet: 10_000,
            wallets_count: 3_000,
            slot_duration: 2,
            slots_in_epoch: 60,
            voting_timing: vec![0, 2, 3],
            block_content_max_size: 102400,
            proposals_count: 1,
            target_resources_usage: ResourcesUsage::new(10, 200_000, 5_000_000),
            tx_target_success_rate: 90,
        }
    }

    pub fn long() -> Self {
        Self {
            name: "long".to_string(),
            rewards_increase: 10u64,
            configuration: Configuration::requests_per_thread(
                5,
                20_000,
                100,
                Monitor::Standard(100),
                100,
            ),
            initial_fund_per_wallet: 10_000,
            wallets_count: 8_000,
            slot_duration: 2,
            slots_in_epoch: 60,
            voting_timing: vec![0, 10, 12],
            block_content_max_size: 102400,
            proposals_count: 1,
            target_resources_usage: ResourcesUsage::new(10, 200_000, 5_000_000),
            tx_target_success_rate: 90,
        }
    }
}

pub struct PrivateVotingLoadTestConfig {
    inner: VotingLoadTestConfig,
    members_count: usize,
    tally_threshold: usize,
}

impl PrivateVotingLoadTestConfig {
    pub fn quick() -> Self {
        Self {
            inner: VotingLoadTestConfig::quick(),
            members_count: 3,
            tally_threshold: 2,
        }
    }

    pub fn long() -> Self {
        Self {
            inner: VotingLoadTestConfig::long(),
            members_count: 10,
            tally_threshold: 8,
        }
    }

    pub fn measurement_name(&self) -> String {
        self.inner.measurement_name("private")
    }

    pub fn rewards_increase(&self) -> u64 {
        self.inner.rewards_increase
    }
    pub fn configuration(&self) -> Configuration {
        self.inner.configuration.clone()
    }
    pub fn initial_fund_per_wallet(&self) -> u64 {
        self.inner.initial_fund_per_wallet
    }
    pub fn wallets_count(&self) -> usize {
        self.inner.wallets_count
    }
    pub fn slot_duration(&self) -> u8 {
        self.inner.slot_duration
    }
    pub fn slots_in_epoch(&self) -> u32 {
        self.inner.slots_in_epoch
    }
    pub fn voting_timing(&self) -> Vec<u32> {
        self.inner.voting_timing.clone()
    }
    pub fn block_content_max_size(&self) -> u32 {
        self.inner.block_content_max_size
    }
    pub fn proposals_count(&self) -> usize {
        self.inner.proposals_count
    }
    pub fn target_resources_usage(&self) -> ResourcesUsage {
        self.inner.target_resources_usage.clone()
    }
    pub fn tx_target_success_rate(&self) -> u32 {
        self.inner.tx_target_success_rate
    }
    pub fn members_count(&self) -> usize {
        self.members_count
    }
    pub fn tally_threshold(&self) -> usize {
        self.tally_threshold
    }
    pub fn total_votes(&self) -> u32 {
        self.inner.total_votes()
    }

    pub fn test_duration(&self) -> std::time::Duration {
        self.inner.test_duration()
    }
}

pub struct PublicVotingLoadTestConfig {
    inner: VotingLoadTestConfig,
}

impl PublicVotingLoadTestConfig {
    pub fn quick() -> Self {
        Self {
            inner: VotingLoadTestConfig::quick(),
        }
    }

    pub fn long() -> Self {
        Self {
            inner: VotingLoadTestConfig::long(),
        }
    }

    pub fn measurement_name(&self) -> String {
        self.inner.measurement_name("public")
    }

    pub fn rewards_increase(&self) -> u64 {
        self.inner.rewards_increase
    }
    pub fn configuration(&self) -> Configuration {
        self.inner.configuration.clone()
    }
    pub fn initial_fund_per_wallet(&self) -> u64 {
        self.inner.initial_fund_per_wallet
    }
    pub fn wallets_count(&self) -> usize {
        self.inner.wallets_count
    }
    pub fn slot_duration(&self) -> u8 {
        self.inner.slot_duration
    }
    pub fn slots_in_epoch(&self) -> u32 {
        self.inner.slots_in_epoch
    }
    pub fn voting_timing(&self) -> Vec<u32> {
        self.inner.voting_timing.clone()
    }
    pub fn block_content_max_size(&self) -> u32 {
        self.inner.block_content_max_size
    }
    pub fn proposals_count(&self) -> usize {
        self.inner.proposals_count
    }
    pub fn target_resources_usage(&self) -> ResourcesUsage {
        self.inner.target_resources_usage.clone()
    }
    pub fn tx_target_success_rate(&self) -> u32 {
        self.inner.tx_target_success_rate
    }
    pub fn total_votes(&self) -> u32 {
        self.inner.total_votes()
    }
    pub fn test_duration(&self) -> std::time::Duration {
        self.inner.test_duration()
    }
}

pub fn adversary_noise_config(tps: usize, duration: Duration) -> Configuration {
    Configuration::duration(tps, duration, 100, Monitor::Disabled(1), 10000)
}
