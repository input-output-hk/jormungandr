use crate::{startup, startup::SingleNodeTestBootstrapper};
use assert_fs::TempDir;
use chain_impl_mockchain::{block::BlockDate, fragment::Fragment};
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{
        explorer::{configuration::ExplorerParams, verifiers::ExplorerVerifier},
        Block0ConfigurationBuilder, NodeConfigBuilder,
    },
};
use jormungandr_lib::interfaces::{ActiveSlotCoefficient, FragmentStatus};
use jortestkit::process::Wait;
use std::{collections::HashMap, time::Duration};
use thor::TransactionHash;

#[test]
pub fn explorer_address_test() {
    let sender = thor::Wallet::default();
    let address_bech32_prefix = sender.address().0;

    let config = Block0ConfigurationBuilder::default()
        .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM);

    let (jormungandr, _initial_stake_pools) =
        startup::start_stake_pool(&[sender.clone()], &[], config, NodeConfigBuilder::default())
            .unwrap();

    let params = ExplorerParams::new(None, None, address_bech32_prefix);
    let explorer_process = jormungandr.explorer(params).unwrap();
    let explorer = explorer_process.client();

    let explorer_address = explorer.address(sender.address().to_string()).unwrap();

    assert!(
        explorer_address.errors.is_none(),
        "{:?}",
        explorer_address.errors.unwrap()
    );

    ExplorerVerifier::assert_address(sender.address(), explorer_address.data.unwrap().address);
}

#[test]
pub fn explorer_transactions_not_existing_address_test() {
    let jcli: JCli = Default::default();
    let sender = thor::Wallet::default();
    let receiver = thor::Wallet::default();
    let test_address = thor::Wallet::default();
    let transaction_value = 1_000;
    let attempts_number = 20;

    let config = Block0ConfigurationBuilder::default()
        .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM);

    let (jormungandr, _initial_stake_pools) = startup::start_stake_pool(
        &[sender.clone()],
        &[sender.clone()],
        config,
        NodeConfigBuilder::default(),
    )
    .unwrap();

    let transaction = thor::FragmentBuilder::from_settings(
        &jormungandr.rest().settings().unwrap(),
        BlockDate::first().next_epoch(),
    )
    .transaction(&sender, receiver.address(), transaction_value.into())
    .unwrap();

    let wait = Wait::new(Duration::from_secs(3), attempts_number);

    jcli.fragment_sender(&jormungandr)
        .send(&transaction.encode())
        .assert_in_block_with_wait(&wait);

    let explorer_process = jormungandr.explorer(ExplorerParams::default()).unwrap();
    let explorer = explorer_process.client();

    let explorer_address = explorer
        .transactions_address(test_address.address().to_string())
        .unwrap();

    assert!(
        explorer_address.errors.is_none(),
        "{:?}",
        explorer_address.errors.unwrap()
    );
    let explorer_transactions_by_address =
        explorer_address.data.unwrap().tip.transactions_by_address;

    ExplorerVerifier::assert_transactions_address(HashMap::new(), explorer_transactions_by_address);
}

// BUG NPG-2869
// TODO comment out the fields (inputs,outputs, certificate) in transaction_by_address.graphql when the bug is fixed
// add the verifier for those fields (inputs,outputs,certificate) in explorer_verifier
#[test]
pub fn explorer_transactions_address_test() {
    let jcli: JCli = Default::default();
    let mut sender = thor::Wallet::default();
    let receiver = thor::Wallet::default();
    let transaction1_value = 1_000;
    let transaction2_value = 2_0;
    let transaction3_value = 3_0;
    let attempts_number = 20;
    let temp_dir = TempDir::new().unwrap();
    let mut fragments = vec![];

    let config =
        Block0ConfigurationBuilder::default().with_utxos(vec![sender.to_initial_fund(1_000_000)]);

    let test_context = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_block0_config(config)
        .build();
    let jormungandr = test_context.start_node(temp_dir).unwrap();

    let wait = Wait::new(Duration::from_secs(3), attempts_number);

    let fragment_builder = thor::FragmentBuilder::from_settings(
        &jormungandr.rest().settings().unwrap(),
        BlockDate::first().next_epoch(),
    );

    let transaction_1 = fragment_builder
        .transaction(&sender, receiver.address(), transaction1_value.into())
        .unwrap();

    jcli.fragment_sender(&jormungandr)
        .send(&transaction_1.encode())
        .assert_in_block_with_wait(&wait);

    fragments.push(&transaction_1);

    sender.confirm_transaction();

    let transaction_2 = fragment_builder
        .transaction(&sender, receiver.address(), transaction2_value.into())
        .unwrap();

    jcli.fragment_sender(&jormungandr)
        .send(&transaction_2.encode())
        .assert_in_block_with_wait(&wait);

    fragments.push(&transaction_2);

    let transaction_3 = fragment_builder
        .transaction(&receiver, sender.address(), transaction3_value.into())
        .unwrap();

    jcli.fragment_sender(&jormungandr)
        .send(&transaction_3.encode())
        .assert_in_block_with_wait(&wait);

    fragments.push(&transaction_3);

    let mut fragments_log = jcli.rest().v0().message().logs(jormungandr.rest_uri());

    fragments_log.sort();
    fragments.sort_by_key(|a| a.hash());

    // make and hashmap of tuples of fragment and fragment status
    let mut fragments_statuses: HashMap<_, _> = fragments
        .iter()
        .zip(fragments_log.iter())
        .map(|(&a, b)| (a.hash().to_string(), (a, b.status())))
        .collect();

    let block0 = test_context.block0_config().to_block();
    let block0fragment: &Fragment = block0.fragments().last().unwrap();
    let block0_fragment_status = FragmentStatus::InABlock {
        date: block0.header().block_date().into(),
        block: block0.header().block_content_hash().into(),
    };
    fragments_statuses.insert(
        block0fragment.hash().to_string(),
        (block0fragment, &block0_fragment_status),
    );

    let explorer_process = jormungandr.explorer(ExplorerParams::default()).unwrap();
    let explorer = explorer_process.client();

    assert!(explorer
        .transactions_address(sender.address().to_string())
        .is_ok());

    let explorer_address = explorer
        .transactions_address(sender.address().to_string())
        .unwrap();

    assert!(
        explorer_address.errors.is_none(),
        "{:?}",
        explorer_address.errors.unwrap()
    );

    let explorer_transactions_by_address =
        explorer_address.data.unwrap().tip.transactions_by_address;

    ExplorerVerifier::assert_transactions_address(
        fragments_statuses,
        explorer_transactions_by_address,
    );
}
