mod context;
mod controller;
pub mod dotifier;
mod fragment_node;
pub mod repository;

pub use self::{
    context::{Context, ContextChaCha},
    controller::{Controller, ControllerBuilder},
};
pub use chain_impl_mockchain::{
    block::Block, chaintypes::ConsensusVersion, header::HeaderId, milli::Milli, value::Value,
};
pub use jormungandr_lib::interfaces::{
    ActiveSlotCoefficient, KesUpdateSpeed, NumberOfSlotsPerEpoch, SlotDuration,
};
pub use jormungandr_testing_utils::testing::network::{
    controller::ControllerError, Blockchain, Node, NodeAlias, Seed, SpawnParams, Topology, Wallet,
    WalletAlias, WalletType,
};
pub use jortestkit::console::progress_bar::{parse_progress_bar_mode_from_str, ProgressBarMode};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Node(#[from] crate::node::Error),

    #[error(transparent)]
    Wallet(#[from] jormungandr_testing_utils::wallet::WalletError),

    #[error(transparent)]
    FsFixture(#[from] assert_fs::fixture::FixtureError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    BlockFormatError(#[from] chain_core::mempack::ReadError),

    #[error("No node with alias {0}")]
    NodeNotFound(String),

    #[error("Wallet '{0}' was not found. Used before or never initialize")]
    WalletNotFound(String),

    #[error("StakePool '{0}' was not found. Used before or never initialize")]
    StakePoolNotFound(String),

    #[error("VotePlan '{0}' was not found. Used before or never initialize")]
    VotePlanNotFound(String),

    #[error(transparent)]
    Controller(#[from] ControllerError),
}

pub type Result<T> = ::core::result::Result<T, Error>;

#[macro_export]
macro_rules! prepare_scenario {
    (
        $title:expr,
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
                $($wallet_type:tt $initial_wallet_name:tt with $initial_wallet_funds:tt $(delegates to $initial_wallet_delegate_to:tt)* ),+ $(,)*
            ] $(,)*
            $(committees = [ $($committe_wallet_name:tt),* $(,)* ] $(,)*)?
            $(legacy = [
                $($legacy_wallet_name:tt address $legacy_wallet_address:tt mnemonics $legacy_wallet_mnemonics:tt with $initial_legacy_funds:tt,)+
            ],)?
            $(vote_plans = [
                $($fund_name:tt from $fund_owner:tt through epochs $vote_start:tt->$vote_tally:tt->$vote_end:tt as $vote_type:tt contains proposals = [
                    $(proposal adds $action_value:tt to $action_target:tt with $proposal_options_count:tt vote options),+ $(,)*
                ]
            )*],)?
        }
    ) => {{
        let mut builder = $crate::scenario::ControllerBuilder::new($title);
        let mut topology = jormungandr_testing_utils::testing::network::Topology::default();
        $(
            #[allow(unused_mut)]
            let mut node = $crate::scenario::Node::new($topology_tt);
            $(
                node = node.with_trusted_peer($node_link);
            )*
            topology = topology.with_node(node);
        )*
        builder = builder.topology(topology);

        let mut blockchain = crate::scenario::Blockchain::default()
            .with_consensus(crate::scenario::ConsensusVersion::$blockchain_consensus)
            .with_consensus_genesis_praos_active_slot_coeff(crate::scenario::ActiveSlotCoefficient::new(crate::scenario::Milli::from_millis(700)).unwrap())
            .with_slots_per_epoch(crate::scenario::NumberOfSlotsPerEpoch::new($slots_per_epoch).unwrap())
            .with_slot_duration(crate::scenario::SlotDuration::new($slot_duration).unwrap());

        $(
            let node_leader = $node_leader.to_owned();
            blockchain = blockchain.with_leader(node_leader);
        )*

        $(
            let wallet = {

                if $wallet_type == "account" {
                    #[allow(unused_mut)]
                    let mut wallet = jormungandr_testing_utils::testing::network::WalletTemplate::new_account(
                        $initial_wallet_name.to_owned(),
                        chain_impl_mockchain::value::Value($initial_wallet_funds).into(),
                        blockchain.discrimination()
                    );

                    $(
                        assert!(
                            wallet.delegate().is_none(),
                            "we only support delegating once for now, fix delegation for wallet \"{}\"",
                            $initial_wallet_name
                        );
                        *wallet.delegate_mut() = Some($initial_wallet_delegate_to.to_owned());
                    )*
                    wallet
                } else if $wallet_type == "utxo" {
                    #[allow(unused_mut)]
                    let wallet = jormungandr_testing_utils::testing::network::WalletTemplate::new_utxo(
                        $initial_wallet_name.to_owned(),
                        chain_impl_mockchain::value::Value($initial_wallet_funds).into(),
                        blockchain.discrimination()
                    );
                    wallet
                } else {
                    panic!("unknown wallet type");
                }
            };
            blockchain = blockchain.with_wallet(wallet);
        )*

        $(
           $(
            blockchain.add_committee($committe_wallet_name.to_owned());
           )*
        )?


        $(
            $(
                let value = chain_impl_mockchain::value::Value($initial_legacy_funds);
                let legacy_wallet = jormungandr_testing_utils::testing::network::LegacyWalletTemplate::new($legacy_wallet_name,value.into(),$legacy_wallet_address.to_owned(),$legacy_wallet_mnemonics.to_owned());
                blockchain.add_legacy_wallet(legacy_wallet);
            )*
        )?

        $(
            $(
                let mut vote_plan_builder = chain_impl_mockchain::testing::scenario::template::VotePlanDefBuilder::new($fund_name);
                vote_plan_builder.owner($fund_owner);

                match $vote_type {
                    "public" => {
                        vote_plan_builder.payload_type(chain_impl_mockchain::vote::PayloadType::Public);
                    }
                    "private" => {
                        vote_plan_builder.payload_type(chain_impl_mockchain::vote::PayloadType::Private);
                    }
                    _ => panic!("unknown vote plan type")
                }

                let vote_start: u32 = $vote_start.to_owned() as u32;
                let vote_tally: u32 = $vote_tally.to_owned() as u32;
                let vote_end: u32 = $vote_end.to_owned() as u32;

                vote_plan_builder.vote_phases(vote_start,vote_tally,vote_end);

                $(
                    let mut proposal_builder = chain_impl_mockchain::testing::scenario::template::ProposalDefBuilder::new(chain_impl_mockchain::testing::VoteTestGen::external_proposal_id());
                    proposal_builder.options($proposal_options_count.into());

                    let action_target = $action_target.to_owned();

                    match action_target.as_str() {
                        "rewards" => {
                            proposal_builder.action_rewards_add($action_value as u64)
                        },
                        "treasury" => {
                            proposal_builder.action_transfer_to_rewards($action_value as u64)
                        },
                        _ => proposal_builder.action_off_chain(),
                    };

                    vote_plan_builder.with_proposal(&mut proposal_builder);
                )*

                blockchain.add_vote_plan(vote_plan_builder.build());
            )*
        )?

        builder.blockchain(blockchain)
    }};
}
