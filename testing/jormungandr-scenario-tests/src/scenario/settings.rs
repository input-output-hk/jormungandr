use crate::scenario::WalletAlias;
use crate::{scenario::Context, style};
use assert_fs::fixture::ChildPath;
use assert_fs::fixture::PathChild;
use chain_impl_mockchain::{
    certificate::VotePlan, testing::create_initial_vote_plan, vote::PayloadType,
};

use jormungandr_lib::crypto::{account::Identifier, key::SigningKey};
use jormungandr_lib::interfaces::try_initials_vec_from_messages;
use jormungandr_lib::{
    interfaces::{
        Explorer, LayersConfig, Mempool, NodeConfig, NodeSecret, P2p, Policy, Rest,
        TopicsOfInterest, VotePlan as VotePlanLib,
    },
    time::Duration,
};
use jormungandr_testing_utils::testing::network::{
    Blockchain as BlockchainTemplate, Node as NodeTemplate, NodeAlias, NodeSetting, Random,
    Settings as NetworkBuilderSettings, Topology as TopologyTemplate, VotePlanKey, WalletTemplate,
    WalletType,
};
use jormungandr_testing_utils::wallet::PrivateVoteCommitteeDataManager;
use rand_core::{CryptoRng, RngCore};
use std::collections::HashMap;
use std::io::Write;

pub trait Prepare: Clone + Send + 'static {
    fn prepare<RNG>(context: &mut Context<RNG>) -> Self
    where
        RNG: RngCore + CryptoRng;
}

pub trait PrepareNodeSettings: Clone + Send {
    fn prepare<RNG>(alias: NodeAlias, context: &mut Context<RNG>, template: &NodeTemplate) -> Self
    where
        RNG: RngCore + CryptoRng;
}

pub trait PrepareVitServerSettings: Clone + Send {
    fn prepare<RNG>(context: &mut Context<RNG>) -> Self
    where
        RNG: RngCore + CryptoRng;
}

pub trait PrepareSettings {
    fn prepare<RNG>(
        topology: TopologyTemplate,
        blockchain: BlockchainTemplate,
        context: &mut Context<RNG>,
    ) -> Self
    where
        RNG: RngCore + CryptoRng;
}

pub type VotePlanAlias = String;

#[derive(Debug)]
pub struct Settings {
    pub network_settings: NetworkBuilderSettings,
    pub private_vote_plans: HashMap<VotePlanAlias, PrivateVoteCommitteeDataManager>,
}

impl Settings {
    pub fn new<RNG>(
        nodes: HashMap<NodeAlias, NodeSetting>,
        blockchain: BlockchainTemplate,
        rng: &mut Random<RNG>,
    ) -> Self
    where
        RNG: RngCore + CryptoRng,
    {
        let network_settings = NetworkBuilderSettings::new(nodes, blockchain.clone(), rng);
        let mut settings = Self {
            network_settings,
            private_vote_plans: HashMap::new(),
        };
        settings.populate_block0_blockchain_vote_plans(
            blockchain.vote_plans(),
            blockchain.committees(),
            rng.rng_mut(),
        );

        println!("{:?}", settings);

        settings
    }

    pub fn dump_private_vote_keys(&self, directory: ChildPath) {
        for (vote_plan_alias, data) in self.private_vote_plans.iter() {
            let (_, vote_plan) = self
                .network_settings
                .vote_plans
                .iter()
                .find(|(alias, _)| *alias == vote_plan_alias)
                .unwrap();
            let vote_plan_dir = directory.child(format!("{}_committees", vote_plan.to_id()));
            data.write_to(vote_plan_dir).unwrap();
        }
    }

    fn populate_block0_blockchain_vote_plans<RNG>(
        &mut self,
        mut vote_plans: HashMap<VotePlanKey, VotePlanLib>,
        committees_aliases: Vec<WalletAlias>,
        rng: &mut RNG,
    ) where
        RNG: RngCore + CryptoRng,
    {
        let mut vote_plans_fragments = Vec::new();

        for (vote_plan_key, vote_plan) in vote_plans.iter_mut() {
            let owner = self
                .network_settings
                .wallets
                .get(&vote_plan_key.owner_alias)
                .unwrap_or_else(|| {
                    panic!(
                        "Owner {} of {} is unknown wallet ",
                        vote_plan_key.owner_alias, vote_plan_key.alias
                    )
                });

            // workaround beacuse vote_plan_def does not expose payload_type
            let tmp_vote_plan: VotePlan = vote_plan.clone().into();

            let vote_plan: VotePlan = match tmp_vote_plan.payload_type() {
                PayloadType::Public => vote_plan.clone().into(),
                PayloadType::Private => {
                    let mut wallets = self.network_settings.wallets.clone();
                    let committees: Vec<(WalletAlias, Identifier)> = committees_aliases
                        .iter()
                        .map(|x| {
                            let wallet = wallets.get_mut(x).unwrap();
                            (x.clone(), wallet.identifier().into())
                        })
                        .collect();
                    let threshold = committees.len();
                    let manager = PrivateVoteCommitteeDataManager::new(rng, committees, threshold);

                    vote_plan
                        .committee_member_public_keys
                        .extend(manager.member_public_keys());
                    self.private_vote_plans
                        .insert(vote_plan_key.alias.clone(), manager);
                    vote_plan.clone().into()
                }
            };

            self.network_settings
                .vote_plans
                .insert(vote_plan_key.alias.clone(), vote_plan.clone());

            vote_plans_fragments.push(create_initial_vote_plan(
                &vote_plan,
                &[owner.clone().into()],
            ));
        }
        self.network_settings
            .block0
            .initial
            .extend(try_initials_vec_from_messages(vote_plans_fragments.iter()).unwrap())
    }
}

pub struct Dotifier;

impl Dotifier {
    pub(crate) fn dottify<W: Write>(&self, settings: &Settings, mut w: W) -> std::io::Result<()> {
        writeln!(&mut w, r"digraph protocol {{")?;

        writeln!(
            &mut w,
            r###"  subgraph nodes {{
    node [ style = filled; color = lightgrey ];
"###
        )?;
        for node in settings.network_settings.nodes.values() {
            let label = self.dot_node_label(node);
            writeln!(&mut w, "    {}", &label)?;

            for trusted_peer in node.node_topology.trusted_peers.iter() {
                let trusted_peer = settings.network_settings.nodes.get(trusted_peer).unwrap();
                writeln!(
                    &mut w,
                    "    {} -> {} [ label = \"trusts\" ; color = blue ]",
                    &label,
                    self.dot_node_label(trusted_peer)
                )?;
            }
        }
        writeln!(&mut w, "  }}")?;

        for wallet in settings.network_settings.wallets.values() {
            let template = wallet.template();
            let label = self.dot_wallet_label(template);
            writeln!(&mut w, "  {}", &label)?;

            if let Some(node) = template.delegate() {
                let trusted_peer = settings.network_settings.nodes.get(node).unwrap();
                writeln!(
                    &mut w,
                    "  {} -> {} [ label = \"delegates\"; style = dashed ; color = red ]",
                    &label,
                    self.dot_node_label(trusted_peer)
                )?;
            }
        }

        writeln!(&mut w, "}}")?;
        Ok(())
    }

    pub(crate) fn dot_wallet_label(&self, wallet: &WalletTemplate) -> String {
        let t: crate::style::icons::Icon = if *wallet.wallet_type() == WalletType::Account {
            *crate::style::icons::account
        } else {
            *crate::style::icons::wallet
        };

        format!("\"{}{}\\nfunds = {}\"", &wallet.alias(), t, wallet.value())
    }

    pub(crate) fn dot_node_label(&self, node_settings: &NodeSetting) -> String {
        let bft = if let Some(_bft) = &node_settings.secret.bft {
            "[b]"
        } else {
            ""
        };

        let genesis = if let Some(_genesis) = &node_settings.secret.genesis {
            "[g]"
        } else {
            ""
        };
        format!(
            "\"{}{}{}{}\"",
            &node_settings.alias,
            *style::icons::jormungandr,
            bft,
            genesis
        )
    }
}

impl PrepareSettings for Settings {
    fn prepare<RNG>(
        topology: TopologyTemplate,
        blockchain: BlockchainTemplate,
        context: &mut Context<RNG>,
    ) -> Self
    where
        RNG: RngCore + CryptoRng,
    {
        let nodes = topology
            .nodes
            .iter()
            .map(|(alias, node)| {
                (
                    alias.clone(),
                    NodeSetting::prepare(alias.clone(), context, node),
                )
            })
            .collect();

        Settings::new(nodes, blockchain, context.random())
    }
}

impl PrepareNodeSettings for NodeSetting {
    fn prepare<RNG>(alias: NodeAlias, context: &mut Context<RNG>, template: &NodeTemplate) -> Self
    where
        RNG: RngCore + CryptoRng,
    {
        NodeSetting {
            alias,
            config: NodeConfig::prepare(context),
            secret: NodeSecret::prepare(context),
            topology_secret: SigningKey::generate(&mut rand::thread_rng()),
            node_topology: template.clone(),
        }
    }
}

impl Prepare for NodeSecret {
    fn prepare<RNG>(_context: &mut Context<RNG>) -> Self
    where
        RNG: RngCore + CryptoRng,
    {
        NodeSecret {
            bft: None,
            genesis: None,
        }
    }
}

impl Prepare for NodeConfig {
    fn prepare<RNG>(context: &mut Context<RNG>) -> Self
    where
        RNG: RngCore + CryptoRng,
    {
        NodeConfig {
            rest: Rest::prepare(context),
            p2p: P2p::prepare(context),
            storage: None,
            log: None,
            mempool: Some(Mempool::prepare(context)),
            explorer: Explorer::prepare(context),
            bootstrap_from_trusted_peers: None,
            skip_bootstrap: None,
        }
    }
}

impl Prepare for Rest {
    fn prepare<RNG>(context: &mut Context<RNG>) -> Self
    where
        RNG: RngCore,
    {
        Rest {
            listen: context.generate_new_rest_listen_address(),
            tls: None,
            cors: None,
        }
    }
}

impl Prepare for Mempool {
    fn prepare<RNG>(_context: &mut Context<RNG>) -> Self
    where
        RNG: RngCore,
    {
        Mempool::default()
    }
}

impl Prepare for Explorer {
    fn prepare<RNG>(_context: &mut Context<RNG>) -> Self
    where
        RNG: RngCore,
    {
        Explorer { enabled: false }
    }
}

impl Prepare for P2p {
    fn prepare<RNG>(context: &mut Context<RNG>) -> Self
    where
        RNG: RngCore + CryptoRng,
    {
        P2p {
            public_address: context.generate_new_grpc_public_address(),
            trusted_peers: Vec::new(),
            allow_private_addresses: true,
            listen: None,
            max_connections: None,
            max_inbound_connections: None,
            policy: Some(Policy::prepare(context)),
            layers: Some(LayersConfig {
                preferred_list: None,
                topics_of_interest: Some(TopicsOfInterest::prepare(context)),
            }),
            node_key_file: None,
            gossip_interval: None,
            max_bootstrap_attempts: None,
            network_stuck_check: None,
        }
    }
}

impl Prepare for TopicsOfInterest {
    fn prepare<RNG>(_context: &mut Context<RNG>) -> Self
    where
        RNG: RngCore,
    {
        TopicsOfInterest {
            messages: "high".to_string(),
            blocks: "high".to_string(),
        }
    }
}

impl Prepare for Policy {
    fn prepare<RNG>(_context: &mut Context<RNG>) -> Self
    where
        RNG: RngCore,
    {
        Policy {
            quarantine_duration: Some(Duration::new(30, 0)),
            quarantine_whitelist: None,
        }
    }
}
