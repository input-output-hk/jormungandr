use assert_fs::TempDir;
use chain_impl_mockchain::fee::LinearFee;
use chain_impl_mockchain::testing::TestGen;
use jormungandr_automation::jormungandr::ConfigurationBuilder;
use jormungandr_automation::jormungandr::Starter;
use jormungandr_lib::interfaces::ActiveSlotCoefficient;
use jormungandr_lib::interfaces::CommitteeIdDef;
use jormungandr_lib::interfaces::Mempool;
use jormungandr_lib::interfaces::SignedCertificate;
//use thor::evm_mapping_cert;
use thor::Wallet;
use crate::startup;
use chain_impl_mockchain::{block::BlockDate, fragment::FragmentId};
use jormungandr_automation::jormungandr::JormungandrProcess;
use jormungandr_automation::jormungandr::MemPoolCheck;
use rstest::*;
use thor::FragmentSender;
use thor::FragmentSenderSetup;



#[test]
pub fn test_evm_mapping() {

    let mut alice = thor::Wallet::default();
    let bob = thor::Wallet::default();

    let (jormungandr, _stake_pools) = startup::start_stake_pool(
        &[alice.clone()],
        &[alice.clone()],
        &mut ConfigurationBuilder::new(),
    )
    .unwrap();

    let transaction_sender = FragmentSender::from(jormungandr.block0_configuration());

    let fragment_builder = thor::FragmentBuilder::new(
        &jormungandr.genesis_block_hash(),
        &jormungandr.fees(),
        BlockDate::first().next_epoch(),
    );

    let evm_mapping = TestGen::evm_mapping_for_wallet(&alice.clone().into());

    let alice_fragment = fragment_builder
        .evm_mapping(&alice, &evm_mapping);

    transaction_sender.send_fragment(&mut alice, alice_fragment.clone(), &jormungandr);

    let log = jormungandr.logger.get_lines_as_string();
    println!("{:?}", log);
/*
    let debug: Vec<String> = log.iter()
    .filter(|x| x.contains(&alice_fragment.hash().to_string()))
    .cloned().collect();
    println!("{:?}", debug);
*/


}