use crate::startup;
use assert_fs::TempDir;
use chain_impl_mockchain::fragment::{FragmentId, Fragment};
use chain_impl_mockchain::key::Hash;
use chain_impl_mockchain::transaction::AccountIdentifier;
use chain_impl_mockchain::{block::BlockDate, transaction};
use jormungandr_automation::jormungandr::Starter;
use jormungandr_automation::jormungandr::explorer::verifier::ExplorerVerifier;
use jormungandr_automation::testing::time;
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{ConfigurationBuilder, Explorer},
};
use jormungandr_lib::interfaces::ActiveSlotCoefficient;
use jortestkit::process::Wait;

use jormungandr_automation::jormungandr::explorer::data::transaction_by_id::TransactionByIdTransactionCertificate;
use jormungandr_automation::jormungandr::explorer::data::TransactionById;
use std::time::Duration;
use std::{borrow::Borrow, str::FromStr};
use thor::{FragmentBuilder, FragmentSender, StakePool, TransactionHash};

/*use async_graphql::{Context, FieldResult, Object, Union};
use async_graphql::Response;
use self::{
    client::GraphQlClient,
    data::{
        address, all_blocks, all_stake_pools, all_vote_plans, blocks_by_chain_length, epoch,
        last_block, settings, stake_pool, transaction_by_id, Address, AllBlocks, AllStakePools,
        AllVotePlans, BlocksByChainLength, Epoch, LastBlock, Settings, StakePool, TransactionById,
    },
};
use graphql_client::GraphQLQuery;
use graphql_client::*;*/

#[test]
pub fn explorer_stake_pool_certificates_test() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let mut first_stake_pool_owner = thor::Wallet::default();
    let mut second_stake_pool_owner = thor::Wallet::default();
    let mut full_delegator = thor::Wallet::default();
    let mut split_delegator = thor::Wallet::default();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![
            first_stake_pool_owner.to_initial_fund(1_000_000),
            second_stake_pool_owner.to_initial_fund(2_000_000),
            full_delegator.to_initial_fund(2_000_000),
            split_delegator.to_initial_fund(2_000_000),
        ])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .expect("cannot start jormungandr");

    let fragment_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        BlockDate::first().next_epoch().into(),
        Default::default(),
    );

    let fragment_builder = FragmentBuilder::new(
        &jormungandr.genesis_block_hash(),
        &jormungandr.fees(),
        BlockDate::first().next_epoch(),
    );

    let explorer_process = jormungandr.explorer();
    let explorer = explorer_process.client();

    let first_stake_pool = StakePool::new(&first_stake_pool_owner);
    let second_stake_pool = StakePool::new(&second_stake_pool_owner);

    // 1). send pool registration certificate

    let stake_pool_reg_fragment =
        fragment_builder.stake_pool_registration(&first_stake_pool_owner, &first_stake_pool);

    fragment_sender
        .send_fragment(
            &mut first_stake_pool_owner,
            stake_pool_reg_fragment.clone(),
            &jormungandr,
        )
        .expect("error while sending registration certificate for first stake pool owner");

    let stake_pool_reg_transaction = explorer
        .transaction(stake_pool_reg_fragment.hash().into())
        .expect("non existing stake pool registration transaction")
        .data
        .unwrap();

    //TODO
    verify_transaction();
    if let Fragment::PoolRegistration(f) = stake_pool_reg_fragment {f;}

    let cert = stake_pool_reg_transaction.transaction.certificate.unwrap();
    ExplorerVerifier::assert_transaction_certificate( cert);

/*
    if let TransactionByIdTransactionCertificate::PoolRegistration(cert) = cert {
        println!("{:?}", cert);

    };
*/
    /*
    // 2. send owner delegation certificat
    let owner_deleg_fragment =
        fragment_builder.owner_delegation(&first_stake_pool_owner, &first_stake_pool);

    fragment_sender
        .send_fragment(
            &mut first_stake_pool_owner,
            owner_deleg_fragment,
            &jormungandr,
        )
        .expect("error while sending owner delegation cert");

    let owner_deleg_transaction = explorer
        .transaction(stake_pool_reg_fragment.hash().into())
        .expect("non existing owner delegation transaction")
        .data
        .unwrap();

    let cert = owner_deleg_transaction.transaction.certificate.unwrap();

    //if let TransactionByIdTransactionCertificate::OwnerStakeDelegation(cert) = cert {println!("value: {:?}", cert.pools);};
    println!("value2: {:?}", cert);


    // 3. send full delegation certificate
    let full_deleg_fragment = fragment_builder.delegation(&full_delegator, &first_stake_pool);

    fragment_sender
        .send_fragment(
            &mut full_delegator,
            full_deleg_fragment.clone(),
            &jormungandr,
        )
        .unwrap();

    let stake_pool_reg_transaction = explorer
        .transaction(full_deleg_fragment.hash().into())
        .expect("non existing full delegation transaction")
        .data
        .unwrap();

    let cert = stake_pool_reg_transaction.transaction.certificate.unwrap();

        //if let TransactionByIdTransactionCertificate::StakeDelegation(cert) = cert {println!("value: {:?}", cert);};
        println!("value3: {:?}", cert);


    // 4. send split delegation certificate
    let split_delegation_fragment = fragment_builder.delegation_to_many(
        &split_delegator,
        vec![(&first_stake_pool, 1u8), (&second_stake_pool, 1u8)],
    );

    fragment_sender
        .send_fragment(
            &mut split_delegator,
            split_delegation_fragment.clone(),
            &jormungandr,
        )
        .unwrap();

    let split_delegation_transaction = explorer
        .transaction(split_delegation_fragment.hash().into())
        .expect("non split delegation transaction")
        .data
        .unwrap();

    let cert = split_delegation_transaction.transaction.certificate.unwrap();

    println!("value4: {:?}", cert);


    // 5. send pool update certificate
    let mut new_stake_pool = first_stake_pool.clone();
    let mut stake_pool_info = new_stake_pool.info_mut();

    stake_pool_info.reward_account = Some(AccountIdentifier::Single(
        second_stake_pool_owner
            .identifier()
            .into_public_key()
            .into(),
    ));

    //time::wait_for_epoch(2, jormungandr.rest());
    let stake_pool_update_fragment = fragment_builder.stake_pool_update(
        vec![&first_stake_pool_owner],
        &first_stake_pool,
        &new_stake_pool,
    );

    jcli.fragment_sender(&jormungandr)
        .send(&stake_pool_update_fragment.encode())
        .assert_in_block();
    first_stake_pool_owner.confirm_transaction();

    let stake_pool_update_transaction = explorer
        .transaction(stake_pool_update_fragment.hash().into())
        .expect("non stake pool update transaction")
        .data
        .unwrap();

    let cert = stake_pool_update_transaction.transaction.certificate.unwrap();

    println!("value5: {:?}", cert);

    // 6. send pool retire certificate
    let stake_pool_retire_fragment =
        fragment_builder.stake_pool_retire(vec![&first_stake_pool_owner], &first_stake_pool);

    fragment_sender
        .send_fragment(
            &mut first_stake_pool_owner,
            stake_pool_retire_fragment.clone(),
            &jormungandr,
        )
        .unwrap();

    let stake_pool_retire_transaction = explorer
        .transaction(stake_pool_retire_fragment.hash().into())
        .expect("non stake pool update transaction")
        .data
        .unwrap();

    let cert = stake_pool_retire_transaction.transaction.certificate.unwrap();

    println!("value6: {:?}", cert);*/
}

#[test]
pub fn explorer_vote_certificates_test() {}

#[test]
pub fn explorer_evm_mapping_certificates_test() {}

fn verify_transaction() {}
