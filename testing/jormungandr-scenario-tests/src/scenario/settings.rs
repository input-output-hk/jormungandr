use crate::{scenario::Context, style};
use jormungandr_lib::{
    interfaces::{Explorer, Mempool, NodeConfig, NodeSecret, P2p, Policy, Rest, TopicsOfInterest},
    time::Duration,
};
use jormungandr_testing_utils::testing::network_builder::{
    Blockchain as BlockchainTemplate, Node as NodeTemplate, NodeAlias, NodeSetting, Settings,
    Topology as TopologyTemplate, WalletTemplate, WalletType,
};
use rand_core::{CryptoRng, RngCore};
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

pub trait PrepareSettings {
    fn prepare<RNG>(
        topology: TopologyTemplate,
        blockchain: BlockchainTemplate,
        context: &mut Context<RNG>,
    ) -> Self
    where
        RNG: RngCore + CryptoRng;
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
        for node in settings.nodes.values() {
            let label = self.dot_node_label(&node);
            writeln!(&mut w, "    {}", &label)?;

            for trusted_peer in node.node_topology.trusted_peers() {
                let trusted_peer = settings.nodes.get(trusted_peer).unwrap();
                writeln!(
                    &mut w,
                    "    {} -> {} [ label = \"trusts\" ; color = blue ]",
                    &label,
                    self.dot_node_label(trusted_peer)
                )?;
            }
        }
        writeln!(&mut w, "  }}")?;

        for wallet in settings.wallets.values() {
            let template = wallet.template();
            let label = self.dot_wallet_label(&template);
            writeln!(&mut w, "  {}", &label)?;

            if let Some(node) = template.delegate() {
                let trusted_peer = settings.nodes.get(node).unwrap();
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
            .into_iter()
            .map(|(alias, template)| {
                (
                    alias.clone(),
                    NodeSetting::prepare(alias, context, &template),
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
            listen_address: None,
            max_connections: None,
            max_inbound_connections: None,
            topics_of_interest: Some(TopicsOfInterest::prepare(context)),
            policy: Some(Policy::prepare(context)),
            layers: None,
            public_id: None,
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
