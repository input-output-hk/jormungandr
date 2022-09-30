pub(crate) mod node;
pub mod vote_plan;
pub(crate) mod wallet;

pub use crate::{builder::settings::node::NodeSetting, config::Blockchain};
use crate::{
    builder::{
        committee::generate_committee_data, explorer::generate_explorer, stake_pool,
        vote::generate_vote_plans, wallet::generate, Node as NodeTemplate, Random, VotePlanKey,
        VotePlanSettings, Wallet,
    },
    config::{CommitteeTemplate, ExplorerTemplate, VotePlanTemplate, WalletTemplate},
};
use assert_fs::fixture::{ChildPath, PathChild};
use chain_crypto::Ed25519;
use chain_impl_mockchain::{chaintypes::ConsensusVersion, fee::LinearFee};
use jormungandr_automation::jormungandr::{
    explorer::configuration::ExplorerConfiguration, NodeAlias,
};
use jormungandr_lib::{
    crypto::key::SigningKey,
    interfaces::{
        try_initial_fragment_from_message, Bft, Block0Configuration, BlockchainConfiguration,
        CommitteeIdDef, NodeId, TrustedPeer,
    },
};
use rand_core::{CryptoRng, RngCore};
use std::collections::{HashMap, HashSet};
use thor::StakePool;

#[derive(Debug, Clone)]
pub struct Settings {
    pub nodes: HashMap<NodeAlias, NodeSetting>,
    pub wallets: Vec<Wallet>,
    pub committees: Vec<CommitteeIdDef>,
    pub block0: Block0Configuration,
    pub explorer: Option<ExplorerConfiguration>,
    pub stake_pools: HashMap<NodeAlias, StakePool>,
    pub vote_plans: HashMap<VotePlanKey, VotePlanSettings>,
}

impl Settings {
    pub fn new<RNG>(
        nodes: HashMap<NodeAlias, NodeSetting>,
        blockchain: &Blockchain,
        wallets: &[WalletTemplate],
        committees: &[CommitteeTemplate],
        explorer: &Option<ExplorerTemplate>,
        vote_plans: &[VotePlanTemplate],
        rng: &mut Random<RNG>,
    ) -> Result<Self, Error>
    where
        RNG: RngCore + CryptoRng,
    {
        let mut settings = Self {
            nodes,
            wallets: Vec::new(),
            committees: Vec::new(),
            block0: Block0Configuration {
                blockchain_configuration: BlockchainConfiguration::new(
                    chain_addr::Discrimination::Test,
                    ConsensusVersion::Bft,
                    LinearFee::new(1, 2, 3),
                ),
                initial: Vec::new(),
            },
            explorer: None,
            stake_pools: HashMap::new(),
            vote_plans: HashMap::new(),
        };

        settings.populate_trusted_peers();
        settings.populate_block0_blockchain_initials(wallets)?;
        let mut data_manager = generate_committee_data(&settings.wallets, committees)?;
        let (vote_plans, fragments) =
            generate_vote_plans(&settings.wallets, vote_plans, &mut data_manager);

        settings.vote_plans = vote_plans;
        let discrimination = settings.block0.blockchain_configuration.discrimination;

        if let Some(explorer) = explorer {
            settings.explorer = Some(generate_explorer(&settings.nodes, explorer)?);
        }

        settings.block0.initial.extend(
            fragments
                .iter()
                .map(|message| try_initial_fragment_from_message(discrimination, message).unwrap()),
        );
        settings.committees = data_manager.committee_ids();
        settings.populate_block0_blockchain_configuration(blockchain, rng);
        Ok(settings)
    }

    fn populate_block0_blockchain_configuration<RNG>(
        &mut self,
        blockchain: &Blockchain,
        rng: &mut Random<RNG>,
    ) where
        RNG: RngCore + CryptoRng,
    {
        let mut blockchain_configuration = &mut self.block0.blockchain_configuration;

        // TODO blockchain_configuration.block0_date = ;
        blockchain_configuration.linear_fees = blockchain.linear_fee();
        blockchain_configuration.discrimination = blockchain.discrimination();
        blockchain_configuration.block0_date = blockchain.block0_date();
        blockchain_configuration.block0_consensus = *blockchain.consensus();
        blockchain_configuration.consensus_leader_ids = {
            if blockchain.has_external_consensus_leader_ids() {
                blockchain.external_consensus_leader_ids()
            } else {
                let mut leader_ids = Vec::new();
                for leader_alias in blockchain.leaders() {
                    let identifier = if let Some(node) = self.nodes.get_mut(leader_alias) {
                        if let Some(bft) = &node.secret.bft {
                            bft.signing_key.identifier()
                        } else {
                            let signing_key = SigningKey::generate(rng.rng_mut());
                            let identifier = signing_key.identifier();
                            node.secret.bft = Some(Bft { signing_key });
                            identifier
                        }
                    } else {
                        SigningKey::<Ed25519>::generate(rng.rng_mut()).identifier()
                    };
                    leader_ids.push(identifier.into());
                }
                leader_ids
            }
        };
        blockchain_configuration.committees = self.committees.clone();
        blockchain_configuration.slots_per_epoch = *blockchain.slots_per_epoch();
        blockchain_configuration.slot_duration = *blockchain.slot_duration();
        blockchain_configuration.tx_max_expiry_epochs = blockchain.tx_max_expiry_epochs();
        blockchain_configuration.treasury = Some(1_000_000.into());
        blockchain_configuration.block_content_max_size = *blockchain.block_content_max_size();
        blockchain_configuration.kes_update_speed = *blockchain.kes_update_speed();
        blockchain_configuration.consensus_genesis_praos_active_slot_coeff =
            *blockchain.consensus_genesis_praos_active_slot_coeff();
    }

    fn populate_block0_blockchain_initials(
        &mut self,
        wallet_templates: &[WalletTemplate],
    ) -> Result<(), Error> {
        let (wallets, wallet_intitials) = generate(wallet_templates)?;
        let (stake_initials, stake_pools) = stake_pool::generate(&wallets, &mut self.nodes)?;
        self.block0.initial.extend(wallet_intitials);
        self.block0.initial.extend(stake_initials);
        self.stake_pools = stake_pools;
        self.wallets = wallets;
        Ok(())
    }

    #[allow(deprecated)]
    fn populate_trusted_peers(&mut self) {
        //generate public id for all nodes treated as trusted peers
        let mut trusted_peers_aliases = HashSet::new();

        //gather aliases which are trusted peers
        for (_alias, node) in self.nodes.iter() {
            for trusted_peer in node.node_topology.trusted_peers.iter() {
                trusted_peers_aliases.insert(trusted_peer.clone());
            }
        }

        let nodes = self.nodes.clone();
        for (_alias, node) in self.nodes.iter_mut() {
            let mut trusted_peers = Vec::new();

            for trusted_peer in node.node_topology.trusted_peers.iter() {
                let trusted_peer = nodes.get(trusted_peer).unwrap();
                let id = NodeId::from(
                    <chain_crypto::SecretKey<chain_crypto::Ed25519>>::generate(rand::thread_rng())
                        .to_public(),
                );
                trusted_peers.push(TrustedPeer {
                    address: trusted_peer.config.p2p.public_address.clone(),
                    id: Some(id),
                })
            }

            node.config.skip_bootstrap = Some(trusted_peers.is_empty());
            node.config.bootstrap_from_trusted_peers = Some(!trusted_peers.is_empty());
            node.config.p2p.trusted_peers = trusted_peers;
        }
    }

    pub fn dump_private_vote_keys(&self, directory: ChildPath) {
        for (vote_plan_alias, data) in self.vote_plans.iter() {
            if let VotePlanSettings::Private { keys, vote_plan: _ } = data {
                let (_, vote_plan) = self
                    .vote_plans
                    .iter()
                    .find(|(alias, _)| *alias == vote_plan_alias)
                    .unwrap();

                let vote_plan_dir = directory.child(format!("{}_committees", vote_plan.to_id()));
                keys.write_to(&vote_plan_dir);
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Settings(#[from] wallet::Error),
    #[error(transparent)]
    Committee(#[from] crate::builder::committee::Error),
    #[error(transparent)]
    Explorer(#[from] crate::builder::explorer::Error),
}
