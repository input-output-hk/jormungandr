use crate::startup;
use chain_core::{
    packer::Codec,
    property::{Deserialize, FromStr},
};
use chain_impl_mockchain::{
    block::Block,
    chaintypes::ConsensusType,
    fee::LinearFee,
    tokens::{identifier::TokenIdentifier, minting_policy::MintingPolicy},
};
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{
        explorer::{configuration::ExplorerParams, verifiers::ExplorerVerifier},
        ConfigurationBuilder, MemPoolCheck, Starter,
    },
    testing::time,
};
use jormungandr_lib::interfaces::{
    ActiveSlotCoefficient, BlockDate, FragmentStatus, InitialToken, Mempool,
};
use mjolnir::generators::FragmentGenerator;
use std::time::Duration;
use thor::{FragmentSender, FragmentSenderSetup, FragmentVerifier};

const BLOCK_QUERY_COMPLEXITY_LIMIT: u64 = 150;
const BLOCK_QUERY_DEPTH_LIMIT: u64 = 30;

#[test]
pub fn explorer_block_test() {
    let jcli: JCli = Default::default();
    let sender = thor::Wallet::default();
    let receiver = thor::Wallet::default();

    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        &[receiver.clone()],
        ConfigurationBuilder::new()
            .with_block0_consensus(ConsensusType::GenesisPraos)
            .with_slots_per_epoch(20)
            .with_block_content_max_size(100000.into())
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(3)
            .with_linear_fees(LinearFee::new(1, 1, 1))
            .with_mempool(Mempool {
                pool_max_entries: 1_000_000usize.into(),
                log_max_entries: 1_000_000usize.into(),
                persistent_log: None,
            })
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
    .unwrap();

    let fragment_sender = FragmentSender::from_with_setup(
        jormungandr.block0_configuration(),
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
        mem_checks.clone(),
        &jormungandr,
    )
    .unwrap();

    let fragments_log = jcli.rest().v0().message().logs(jormungandr.rest_uri());
    let fragment_log = fragments_log
        .iter()
        .find(|x| {
            *x.fragment_id().to_string() == mem_checks.last().unwrap().fragment_id().to_string()
        })
        .unwrap();

    let fragment_block_id =
        if let &FragmentStatus::InABlock { date: _, block } = fragment_log.status() {
            block
        } else {
            panic!("Fragment not in block")
        };

    let encoded_block = jcli
        .rest()
        .v0()
        .block()
        .get(fragment_block_id.to_string(), jormungandr.rest_uri());

    let bytes_block = hex::decode(encoded_block.trim()).unwrap();
    let reader = std::io::Cursor::new(&bytes_block);
    let decoded_block = Block::deserialize(&mut Codec::new(reader)).unwrap();

    let params = ExplorerParams::new(BLOCK_QUERY_COMPLEXITY_LIMIT, BLOCK_QUERY_DEPTH_LIMIT, None);
    let explorer_process = jormungandr.explorer(params).unwrap();
    let explorer = explorer_process.client();

    let explorer_block_response = explorer.block_by_id(fragment_block_id.to_string()).unwrap();

    assert!(
        explorer_block_response.errors.is_none(),
        "{:?}",
        explorer_block_response.errors.unwrap()
    );

    let explorer_block = explorer_block_response.data.unwrap().block;

    ExplorerVerifier::assert_block_by_id(decoded_block, explorer_block).unwrap();
}

#[should_panic]
#[test] //NPG-3274
pub fn explorer_block0_test() {
    let jormungandr = Starter::new().start().unwrap();
    let block0_id = jormungandr.genesis_block_hash().to_string();
    let params = ExplorerParams::new(BLOCK_QUERY_COMPLEXITY_LIMIT, BLOCK_QUERY_DEPTH_LIMIT, None);
    let explorer_process = jormungandr.explorer(params).unwrap();
    let explorer = explorer_process.client();

    let explorer_block0_response = explorer.block_by_id(block0_id).unwrap();

    assert!(
        explorer_block0_response.errors.is_none(),
        "{:?}",
        explorer_block0_response.errors.unwrap()
    );

    let explorer_block0 = explorer_block0_response.data.unwrap().block;
    let block0 = jormungandr.block0_configuration().to_block();
    ExplorerVerifier::assert_block_by_id(block0, explorer_block0).unwrap();
}

#[test]
pub fn explorer_block_incorrect_id_test() {
    let incorrect_block_ids = vec![
        (
            "e1049ea45726f0b1fc473af54f706546b3331765abf89ae9e6a8333e49621641aa",
            "invalid hash size",
        ),
        (
            "e1049ea45726f0b1fc473af54f706546b3331765abf89ae9e6a8333e49621641a",
            "invalid hex encoding",
        ),
        (
            "e1049ea45726f0b1fc473af54f706546b3331765abf89ae9e6a8333e49621641",
            "Couldn't find block in the explorer",
        ),
    ];

    let jormungandr = Starter::new().start().unwrap();

    let params = ExplorerParams::new(BLOCK_QUERY_COMPLEXITY_LIMIT, BLOCK_QUERY_DEPTH_LIMIT, None);
    let explorer_process = jormungandr.explorer(params).unwrap();
    let explorer = explorer_process.client();

    for (incorrect_block_id, error_message) in incorrect_block_ids {
        let response = explorer.block_by_id(incorrect_block_id.to_string());
        assert!(response.as_ref().unwrap().errors.is_some());
        assert!(response.as_ref().unwrap().data.is_none());
        assert!(response
            .unwrap()
            .errors
            .unwrap()
            .first()
            .unwrap()
            .message
            .contains(error_message));
    }
}
