use crate::startup::SingleNodeTestBootstrapper;
use assert_fs::TempDir;
use chain_core::property::FromStr;
use chain_impl_mockchain::{
    chaintypes::ConsensusType,
    fee::LinearFee,
    tokens::{identifier::TokenIdentifier, minting_policy::MintingPolicy},
};
use jormungandr_automation::{
    jormungandr::{Block0ConfigurationBuilder, MemPoolCheck, NodeConfigBuilder},
    testing::time,
};
use jormungandr_lib::interfaces::{ActiveSlotCoefficient, BlockDate, InitialToken, Mempool};
use mjolnir::generators::FragmentGenerator;
use std::time::Duration;
use thor::{
    Block0ConfigurationBuilderExtension, FragmentSender, FragmentSenderSetup, FragmentVerifier,
};
#[test]
pub fn send_all_fragments() {
    let temp_dir = TempDir::new().unwrap();
    let receiver = thor::Wallet::default();
    let sender = thor::Wallet::default();
    let stake_pool = thor::StakePool::new(&sender);

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_genesis_praos_stake_pool(&stake_pool)
        .with_block0_config(
            Block0ConfigurationBuilder::minimal_setup()
                .with_wallets_having_some_values(vec![&sender, &receiver])
                .with_stake_pool_and_delegation(&stake_pool, vec![&sender])
                .with_block0_consensus(ConsensusType::GenesisPraos)
                .with_slots_per_epoch(20.try_into().unwrap())
                .with_block_content_max_size(100000.into())
                .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
                .with_slot_duration(3.try_into().unwrap())
                .with_linear_fees(LinearFee::new(1, 1, 1))
                .with_token(InitialToken {
                    // FIXME: this works because I know it's the VotePlanBuilder's default, but
                    // probably should me more explicit.
                    token_id: TokenIdentifier::from_str(
                        "00000000000000000000000000000000000000000000000000000000.00000000",
                    )
                    .unwrap()
                    .into(),
                    policy: MintingPolicy::new().into(),
                    to: vec![sender.to_initial_token(1_000_000)],
                }),
        )
        .with_node_config(NodeConfigBuilder::default().with_mempool(Mempool {
            pool_max_entries: 1_000_000usize.into(),
            log_max_entries: 1_000_000usize.into(),
            persistent_log: None,
        }))
        .build()
        .start_node(temp_dir)
        .unwrap();

    let fragment_sender = FragmentSender::from_settings_with_setup(
        &jormungandr.rest().settings().unwrap(),
        FragmentSenderSetup::no_verify(),
    );

    let time_era = jormungandr.time_era();

    let mut fragment_generator = FragmentGenerator::new(
        sender,
        receiver,
        None,
        jormungandr.to_remote(),
        time_era.slots_per_epoch(),
        2,
        2,
        2,
        0,
        fragment_sender,
    );

    fragment_generator.prepare(BlockDate::new(1, 0));

    time::wait_for_epoch(2, jormungandr.rest());

    let mem_checks: Vec<MemPoolCheck> = fragment_generator.send_all().unwrap();

    FragmentVerifier::wait_and_verify_all_are_in_block(
        Duration::from_secs(2),
        mem_checks,
        &jormungandr,
    )
    .unwrap();
}
