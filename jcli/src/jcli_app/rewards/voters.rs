use super::Error;
use crate::jcli_app::{
    block::Common,
    utils::io::{open_file_read, open_file_write},
};

use structopt::StructOpt;

use chain_crypto::{Ed25519, PublicKey};
use chain_impl_mockchain::vote::CommitteeId;
use jormungandr_lib::interfaces::{
    Address, Block0Configuration, Block0ConfigurationError, Initial, Value,
};
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::ops::Div;
use std::str::FromStr;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct VotersRewards {
    #[structopt(flatten)]
    common: Common,
}

fn calculate_stake<'address>(
    committee_keys: &HashSet<Address>,
    block0: &'address Block0Configuration,
) -> (u64, HashMap<&'address Address, u64>) {
    let mut total_stake: u64 = 0;
    let mut stake_per_voter: HashMap<&'address Address, u64> = HashMap::new();

    for fund in &block0.initial {
        match fund {
            Initial::Fund(fund) => {
                for utxo in fund {
                    if !committee_keys.contains(&utxo.address) {
                        let value: u64 = utxo.value.into();
                        total_stake += value;
                        let entry = stake_per_voter.entry(&utxo.address).or_default();
                        *entry += value;
                    }
                }
            }
            Initial::Cert(_) => {}
            Initial::LegacyFund(_) => {}
        }
    }
    (total_stake, stake_per_voter)
}

fn calculate_reward<'address>(
    total_stake: u64,
    stake_per_voter: &HashMap<&'address Address, u64>,
) -> HashMap<&'address Address, u64> {
    stake_per_voter
        .iter()
        .map(|(k, v)| (*k, v.div(total_stake)))
        .collect()
}

fn write_rewards_results(common: Common, results: HashMap<&Address, u64>) -> Result<(), Error> {
    let writer = common.open_output()?;
    let header = [
        "Address",
        "Stake of the voter",
        "Reward for the voter (ADA)",
        "Reward for the voter (lovelace)",
    ];
    let csv_writer = csv::Writer::from_writer(writer);
    Ok(())
}

impl VotersRewards {
    pub fn exec(self) -> Result<(), Error> {
        let VotersRewards { common } = self;
        let block = common.input.load_block()?;
        let block0 = Block0Configuration::from_block(&block)
            .map_err(Error::BuildingGenesisFromBlock0Failed)?;
        let committee_keys: HashSet<Address> = block0
            .blockchain_configuration
            .committees
            .iter()
            .cloned()
            .map(|id| {
                let pk = CommitteeId::from(id).public_key();
                chain_addr::Address::from_bytes(pk.as_ref()).unwrap().into()
            })
            .collect();

        let (total_stake, stake_per_voter) = calculate_stake(&committee_keys, &block0);
        let rewards = calculate_reward(total_stake, &stake_per_voter);
        write_rewards_results(common, rewards)?;
        Ok(())
    }
}
