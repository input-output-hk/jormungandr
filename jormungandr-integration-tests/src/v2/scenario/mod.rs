mod blockchain;
pub mod settings;
mod topology;
mod wallet;

pub use self::{
    blockchain::Blockchain,
    topology::{Node, NodeAlias, Topology, TopologyBuilder},
    wallet::{Wallet, WalletAlias, WalletType},
};
pub use chain_impl_mockchain::{
    block::{Block, ConsensusVersion, HeaderHash},
    value::Value,
};
use mktemp::Temp;
use rand_chacha::ChaChaRng;
use rand_core::{RngCore, SeedableRng};
use std::{collections::BTreeMap, net::SocketAddr};

error_chain! {
    links {
        Node(crate::v2::node::Error, crate::v2::node::ErrorKind);
    }

    foreign_links {
        Io(std::io::Error);
        Reqwest(reqwest::Error);
        BlockFormatErrror(chain_core::mempack::ReadError);
    }

    errors {
        NodeNotFound(node: String) {
            description("Node not found"),
            display("No node with alias {}", node),
        }

        InvalidHeaderHash {
            description("Invalid header hash"),
        }
    }
}

#[macro_export]
macro_rules! prepare_scenario {
    (
        $context:expr,
        topology [
            $($topology_tt:tt $(-> $node_link:tt)*),+ $(,)*
        ]
        blockchain {
            consensus = $blockchain_consensus:tt,
            leaders = [ $($node_leader:tt),* $(,)* ],
            initials = [
                $(account $initial_wallet_name:tt with $initial_wallet_funds:tt $(delegates to $initial_wallet_delegate_to:tt)* ),+ $(,)*
            ] $(,)*
        }
    ) => {{
        let mut topology_builder = $crate::v2::scenario::TopologyBuilder::new();
        $(
            #[allow(unused_mut)]
            let mut node = $crate::v2::scenario::Node::new($topology_tt);
            $(
                node.add_trusted_peer($node_link);
            )*
            topology_builder.register_node(node);
        )*
        let topology : $crate::v2::scenario::Topology = topology_builder.build();

        let mut blockchain = $crate::v2::scenario::Blockchain::new(
            $crate::v2::scenario::ConsensusVersion::$blockchain_consensus
        );

        $(
            let node_leader = $node_leader.to_owned();
            blockchain.add_leader(node_leader);
        )*

        $(
            #[allow(unused_mut)]
            let mut wallet = $crate::v2::scenario::Wallet::new_account(
                $initial_wallet_name.to_owned(),
                $crate::v2::scenario::Value($initial_wallet_funds)
            );

            $(
                assert!(
                    wallet.delegate().is_none(),
                    "we only support delegating once for now, fix delegation for wallet \"{}\"",
                    $initial_wallet_name
                );
                *wallet.delegate_mut() = Some($initial_wallet_delegate_to.to_owned());
            )*


            blockchain.add_wallet(wallet);
        )*

        let settings = $crate::v2::scenario::settings::Settings::prepare(
            topology,
            blockchain,
            $context
        );

        $crate::v2::scenario::Scenario::new(settings)
    }};
}

pub enum NodeBlock0 {
    Hash(HeaderHash),
    File(std::path::PathBuf),
}

pub struct Scenario {
    settings: settings::Settings,

    block0_file: Temp,
    block0_hash: HeaderHash,

    nodes: BTreeMap<NodeAlias, crate::v2::node::Node>,
}

/// scenario context with all the details to setup the necessary port number
/// a pseudo random number generator (and its original seed).
///
pub struct Context<RNG: RngCore + Sized> {
    rng: RNG,

    seed: [u8; 32],

    next_available_rest_port_number: u16,
    next_available_grpc_port_number: u16,
}

impl Scenario {
    pub fn new(settings: settings::Settings) -> Result<Self> {
        use chain_core::property::Serialize as _;
        let block0_file = Temp::new_file()?;

        let file = std::fs::File::create(&block0_file)?;

        let block0 = settings.block0.to_block();
        let block0_hash = block0.header.hash();

        block0.serialize(file)?;

        Ok(Scenario {
            settings,
            block0_file,
            block0_hash,
            nodes: BTreeMap::new(),
        })
    }

    pub fn spawn_node(&mut self, node_alias: &str, with_block0: bool) -> Result<&crate::v2::Node> {
        let node_setting = if let Some(node_setting) = self.settings.nodes.get(node_alias) {
            node_setting
        } else {
            bail!(ErrorKind::NodeNotFound(node_alias.to_owned()))
        };

        let block0_setting = if with_block0 {
            NodeBlock0::File(self.block0_file.as_path().into())
        } else {
            NodeBlock0::Hash(self.block0_hash.clone())
        };
        let node = crate::v2::node::Node::spawn("node1", node_setting, block0_setting)?;

        self.nodes.insert(node_alias.to_owned(), node);

        Ok(self.nodes.get(node_alias).unwrap())
    }

    pub fn get_tip(&self, node_alias: &str) -> Result<HeaderHash> {
        let node_setting = if let Some(node_setting) = self.settings.nodes.get(node_alias) {
            node_setting
        } else {
            bail!(ErrorKind::NodeNotFound(node_alias.to_owned()))
        };

        let address = node_setting.config.rest.listen.clone();
        let hash = reqwest::get(&format!("http://{}/api/v0/tip", address))?.text()?;

        hash.parse().chain_err(|| ErrorKind::InvalidHeaderHash)
    }

    pub fn get_block(&self, node_alias: &str, hash: &HeaderHash) -> Result<Block> {
        use chain_core::mempack::Readable as _;

        let node_setting = if let Some(node_setting) = self.settings.nodes.get(node_alias) {
            node_setting
        } else {
            bail!(ErrorKind::NodeNotFound(node_alias.to_owned()))
        };

        let address = node_setting.config.rest.listen.clone();
        let mut blob = Vec::with_capacity(4096);
        let _size = reqwest::get(&format!("http://{}/api/v0/block/{}", address, hash))?
            .copy_to(&mut blob)?;

        let mut buf = chain_core::mempack::ReadBuf::from(&blob);

        let block = Block::read(&mut buf)?;
        Ok(block)
    }

    pub fn node(&self, node_alias: &str) -> Option<&crate::v2::Node> {
        self.nodes.get(node_alias)
    }

    pub fn node_mut(&mut self, node_alias: &str) -> Option<&mut crate::v2::Node> {
        self.nodes.get_mut(node_alias)
    }
}

impl Context<ChaChaRng> {
    pub fn new() -> Self {
        let mut seed = [0; 32];
        rand::rngs::OsRng::new().unwrap().fill_bytes(&mut seed);
        let rng = ChaChaRng::from_seed(seed);

        Context {
            rng,
            seed,
            next_available_rest_port_number: 11_000,
            next_available_grpc_port_number: 12_000,
        }
    }
}

impl<RNG: RngCore> Context<RNG> {
    pub fn generate_new_rest_listen_address(&mut self) -> SocketAddr {
        use std::net::{IpAddr, Ipv4Addr};

        let port_number = self.next_available_rest_port_number;
        self.next_available_rest_port_number += 1;
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port_number)
    }

    pub fn generate_new_grpc_public_address(&mut self) -> String {
        use std::net::{IpAddr, Ipv4Addr};

        let port_number = self.next_available_grpc_port_number;
        self.next_available_grpc_port_number += 1;

        let address = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        format!("/ip4/{}/tcp/{}", address, port_number)
    }

    /// retrieve the original seed of the pseudo random generator
    #[inline]
    pub fn seed(&self) -> &[u8; 32] {
        &self.seed
    }
}
