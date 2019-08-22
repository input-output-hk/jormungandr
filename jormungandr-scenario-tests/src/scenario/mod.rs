mod blockchain;
mod context;
mod controller;
pub mod settings;
mod topology;
mod wallet;

pub use self::{
    blockchain::Blockchain,
    context::{Context, ContextChaCha, Seed},
    controller::{Controller, ControllerBuilder},
    topology::{Node, NodeAlias, Topology, TopologyBuilder},
    wallet::{Wallet, WalletAlias, WalletType},
};
pub use chain_impl_mockchain::{
    block::{Block, ConsensusVersion, HeaderHash},
    value::Value,
};
pub use jormungandr_lib::interfaces::{NumberOfSlotsPerEpoch, SlotDuration};

error_chain! {
    links {
        Node(crate::node::Error, crate::node::ErrorKind);
    }

    foreign_links {
        Io(std::io::Error);
        Reqwest(reqwest::Error);
        BlockFormatError(chain_core::mempack::ReadError);
    }

    errors {
        NodeNotFound(node: String) {
            description("Node not found"),
            display("No node with alias {}", node),
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
            number_of_slots_per_epoch = $slots_per_epoch:tt,
            slot_duration = $slot_duration:tt,
            leaders = [ $($node_leader:tt),* $(,)* ],
            initials = [
                $(account $initial_wallet_name:tt with $initial_wallet_funds:tt $(delegates to $initial_wallet_delegate_to:tt)* ),+ $(,)*
            ] $(,)*
        }
    ) => {{
        let mut builder = $crate::scenario::ControllerBuilder::new("title...");
        let mut topology_builder = $crate::scenario::TopologyBuilder::new();
        $(
            #[allow(unused_mut)]
            let mut node = $crate::scenario::Node::new($topology_tt);
            $(
                node.add_trusted_peer($node_link);
            )*
            topology_builder.register_node(node);
        )*
        let topology : $crate::scenario::Topology = topology_builder.build();
        builder.set_topology(topology);

        let mut blockchain = $crate::scenario::Blockchain::new(
            $crate::scenario::ConsensusVersion::$blockchain_consensus,
            $crate::scenario::NumberOfSlotsPerEpoch::new($slots_per_epoch).expect("valid number of slots per epoch"),
            $crate::scenario::SlotDuration::new($slot_duration).expect("valid slot duration in seconds"),
        );

        $(
            let node_leader = $node_leader.to_owned();
            blockchain.add_leader(node_leader);
        )*

        $(
            #[allow(unused_mut)]
            let mut wallet = $crate::scenario::Wallet::new_account(
                $initial_wallet_name.to_owned(),
                $crate::scenario::Value($initial_wallet_funds)
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
        builder.set_blockchain(blockchain);

        builder.build_settings($context);

        builder
    }};
}
