use assert_fs::TempDir;
use chain_core::{packer::Codec, property::Deserialize};
use chain_impl_mockchain::{block::Block, header::BlockDate};
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{
        explorer::{configuration::ExplorerParams, verifiers::ExplorerVerifier},
        ConfigurationBuilder, Starter,
    },
};
use jormungandr_lib::interfaces::FragmentStatus;
use jortestkit::process::Wait;
use std::time::Duration;
use thor::{TransactionHash, StakePool};

const BLOCK_QUERY_COMPLEXITY_LIMIT: u64 = 150;
const BLOCK_QUERY_DEPTH_LIMIT: u64 = 30;

#[test]
pub fn explorer_block_test() {
    let jcli: JCli = Default::default();
    let mut sender = thor::Wallet::default();
    let receiver = thor::Wallet::default();
    let stake_pool = StakePool::new(&sender);
    let transaction_value = 1_000;
    let attempts_number = 20;
    let temp_dir = TempDir::new().unwrap();

    let config = ConfigurationBuilder::default()
        .with_funds(vec![sender.to_initial_fund(1_000_000)])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .expect("Cannot start jormungandr");

    let wait = Wait::new(Duration::from_secs(3), attempts_number);

    let fragment_builder = thor::FragmentBuilder::new(
        &jormungandr.genesis_block_hash(),
        &jormungandr.fees(),
        BlockDate::first().next_epoch(),
    );

    let stake_pool_reg_fragment =
    fragment_builder.stake_pool_registration(&sender, &stake_pool);

    jcli.fragment_sender(&jormungandr)
        .send(&stake_pool_reg_fragment.encode())
        .assert_in_block_with_wait(&wait);

    sender.confirm_transaction();


    let transaction = fragment_builder
        .transaction(&sender, receiver.address(), transaction_value.into())
        .unwrap();

    jcli.fragment_sender(&jormungandr)
        .send(&transaction.encode())
        .assert_in_block_with_wait(&wait);

    sender.confirm_transaction();

    let fragments_log = jcli.rest().v0().message().logs(jormungandr.rest_uri());
    let fragment_log = fragments_log
        .iter()
        .find(|x| *x.fragment_id().to_string() == stake_pool_reg_fragment.hash().to_string())
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
    let explorer_process = jormungandr.explorer(params);
    let explorer = explorer_process.client();

    let explorer_block_response = explorer.block(fragment_block_id.to_string()).unwrap();

    assert!(
        explorer_block_response.errors.is_none(),
        "{:?}",
        explorer_block_response.errors.unwrap()
    );

    let explorer_block = explorer_block_response.data.unwrap().block;

    ExplorerVerifier::assert_block(decoded_block, explorer_block).unwrap();
}

#[test] //NPG-3274
pub fn explorer_block0_test() {
    let jormungandr = Starter::new().start().unwrap();
    let block0_id = jormungandr.genesis_block_hash().to_string();
    let params = ExplorerParams::new(BLOCK_QUERY_COMPLEXITY_LIMIT, BLOCK_QUERY_DEPTH_LIMIT, None);
    let explorer_process = jormungandr.explorer(params);
    let explorer = explorer_process.client();

    let explorer_block0_response = explorer.block(block0_id).unwrap();

    assert!(
        explorer_block0_response.errors.is_none(),
        "{:?}",
        explorer_block0_response.errors.unwrap()
    );

    let explorer_block0 = explorer_block0_response.data.unwrap().block;
    let block0 = jormungandr.block0_configuration().to_block();
    ExplorerVerifier::assert_block(block0, explorer_block0).unwrap();
}

#[should_panic] //NPG-2899
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

    let explorer_process = jormungandr.explorer(ExplorerParams::default());
    let explorer = explorer_process.client();

    for (incorrect_block_id, error_message) in incorrect_block_ids {
        let response = explorer.block(incorrect_block_id.to_string());
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
