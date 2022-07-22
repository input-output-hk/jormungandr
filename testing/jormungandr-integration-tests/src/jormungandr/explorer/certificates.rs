use assert_fs::TempDir;
use chain_impl_mockchain::{block::BlockDate, transaction::AccountIdentifier};
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{
        explorer::{configuration::ExplorerParams, verifier::ExplorerVerifier},
        ConfigurationBuilder, Starter,
    },
};
use thor::{FragmentBuilder, FragmentSender, StakePool, TransactionHash};

#[test]
pub fn explorer_stake_pool_registration_test() {
    let query_complexity_limit = 70;
    let query_depth_limit = 30;
    let temp_dir = TempDir::new().unwrap();
    let mut first_stake_pool_owner = thor::Wallet::default();
    let first_stake_pool = StakePool::new(&first_stake_pool_owner);
    let config = ConfigurationBuilder::new()
        .with_funds(vec![first_stake_pool_owner.to_initial_fund(1_000_000)])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .expect("Cannot start jormungandr");

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

    let params = ExplorerParams::new(
        query_complexity_limit.to_string(),
        query_depth_limit.to_string(),
        None,
    );
    let explorer_process = jormungandr.explorer(params);
    let explorer = explorer_process.client();

    let stake_pool_reg_fragment =
        fragment_builder.stake_pool_registration(&first_stake_pool_owner, &first_stake_pool);

    fragment_sender
        .send_fragment(
            &mut first_stake_pool_owner,
            stake_pool_reg_fragment.clone(),
            &jormungandr,
        )
        .expect("Error while sending registration certificate for first stake pool owner");

    let trans = explorer
        .transaction_certificates(stake_pool_reg_fragment.hash().into())
        .expect("Non existing stake pool registration transaction");

    assert!(trans.errors.is_none(), "{:?}", trans.errors.unwrap());

    let exp_stake_pool_reg_transaction = trans.data.unwrap().transaction;

    ExplorerVerifier::assert_transaction_certificates(
        stake_pool_reg_fragment,
        exp_stake_pool_reg_transaction,
    )
    .unwrap();
}

#[test]
pub fn explorer_owner_delegation_test() {
    let query_complexity_limit = 70;
    let query_depth_limit = 30;
    let temp_dir = TempDir::new().unwrap();
    let mut stake_pool_owner = thor::Wallet::default();
    let stake_pool = StakePool::new(&stake_pool_owner);

    let config = ConfigurationBuilder::new()
        .with_funds(vec![stake_pool_owner.to_initial_fund(1_000_000)])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .expect("Cannot start jormungandr");

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

    let stake_pool_reg_fragment =
        fragment_builder.stake_pool_registration(&stake_pool_owner, &stake_pool);

    fragment_sender
        .send_fragment(&mut stake_pool_owner, stake_pool_reg_fragment, &jormungandr)
        .expect("Error while sending registration certificate for stake pool owner");

    let params = ExplorerParams::new(
        query_complexity_limit.to_string(),
        query_depth_limit.to_string(),
        None,
    );
    let explorer_process = jormungandr.explorer(params);
    let explorer = explorer_process.client();

    let owner_deleg_fragment = fragment_builder.owner_delegation(&stake_pool_owner, &stake_pool);

    fragment_sender
        .send_fragment(
            &mut stake_pool_owner,
            owner_deleg_fragment.clone(),
            &jormungandr,
        )
        .expect("Error while sending owner delegation cert");

    let trans = explorer
        .transaction_certificates(owner_deleg_fragment.hash().into())
        .expect("Non existing owner delegation transaction");

    assert!(trans.errors.is_none(), "{:?}", trans.errors.unwrap());

    let owner_deleg_transaction = trans.data.unwrap().transaction;

    ExplorerVerifier::assert_transaction_certificates(
        owner_deleg_fragment,
        owner_deleg_transaction,
    )
    .unwrap();
}

#[test]
pub fn explorer_full_delegation_test() {
    let query_complexity_limit = 70;
    let query_depth_limit = 30;
    let temp_dir = TempDir::new().unwrap();
    let mut stake_pool_owner = thor::Wallet::default();
    let mut full_delegator = thor::Wallet::default();

    let stake_pool = StakePool::new(&stake_pool_owner);

    let config = ConfigurationBuilder::new()
        .with_funds(vec![
            stake_pool_owner.to_initial_fund(1_000_000),
            full_delegator.to_initial_fund(2_000_000),
        ])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .expect("Cannot start jormungandr");

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

    let stake_pool_reg_fragment =
        fragment_builder.stake_pool_registration(&stake_pool_owner, &stake_pool);

    fragment_sender
        .send_fragment(&mut stake_pool_owner, stake_pool_reg_fragment, &jormungandr)
        .expect("Error while sending registration certificate for stake pool owner");

    let params = ExplorerParams::new(
        query_complexity_limit.to_string(),
        query_depth_limit.to_string(),
        None,
    );
    let explorer_process = jormungandr.explorer(params);
    let explorer = explorer_process.client();

    let full_deleg_fragment = fragment_builder.delegation(&full_delegator, &stake_pool);

    fragment_sender
        .send_fragment(
            &mut full_delegator,
            full_deleg_fragment.clone(),
            &jormungandr,
        )
        .unwrap();

    let trans = explorer
        .transaction_certificates(full_deleg_fragment.hash().into())
        .expect("Non existing full delegation transaction");

    assert!(trans.errors.is_none(), "{:?}", trans.errors.unwrap());

    let stake_pool_reg_transaction = trans.data.unwrap().transaction;

    ExplorerVerifier::assert_transaction_certificates(
        full_deleg_fragment,
        stake_pool_reg_transaction,
    )
    .unwrap();
}

#[test]
pub fn explorer_split_delegation_test() {
    let query_complexity_limit = 70;
    let query_depth_limit = 30;
    let temp_dir = TempDir::new().unwrap();
    let mut first_stake_pool_owner = thor::Wallet::default();
    let mut split_delegator = thor::Wallet::default();
    let mut second_stake_pool_owner = thor::Wallet::default();

    let first_stake_pool = StakePool::new(&first_stake_pool_owner);
    let second_stake_pool = StakePool::new(&second_stake_pool_owner);

    let config = ConfigurationBuilder::new()
        .with_funds(vec![
            first_stake_pool_owner.to_initial_fund(1_000_000),
            second_stake_pool_owner.to_initial_fund(1_000_000),
            split_delegator.to_initial_fund(2_000_000),
        ])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .expect("Cannot start jormungandr");

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

    let stake_pool_reg_fragment =
        fragment_builder.stake_pool_registration(&first_stake_pool_owner, &first_stake_pool);

    fragment_sender
        .send_fragment(
            &mut first_stake_pool_owner,
            stake_pool_reg_fragment,
            &jormungandr,
        )
        .expect("Error while sending registration certificate for stake pool owner");

    let stake_pool_reg_fragment =
        fragment_builder.stake_pool_registration(&second_stake_pool_owner, &second_stake_pool);

    fragment_sender
        .send_fragment(
            &mut second_stake_pool_owner,
            stake_pool_reg_fragment,
            &jormungandr,
        )
        .expect("Error while sending registration certificate for stake pool owner");

    let params = ExplorerParams::new(
        query_complexity_limit.to_string(),
        query_depth_limit.to_string(),
        None,
    );
    let explorer_process = jormungandr.explorer(params);
    let explorer = explorer_process.client();

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

    let trans = explorer
        .transaction_certificates(split_delegation_fragment.hash().into())
        .expect("Non existing split delegation transaction");

    assert!(trans.errors.is_none(), "{:?}", trans.errors.unwrap());

    let exp_split_delegation_transaction = trans.data.unwrap().transaction;

    ExplorerVerifier::assert_transaction_certificates(
        split_delegation_fragment,
        exp_split_delegation_transaction,
    )
    .unwrap();
}

#[test]
pub fn explorer_pool_update_test() {
    let query_complexity_limit = 70;
    let query_depth_limit = 30;
    let jcli: JCli = Default::default();
    let temp_dir = TempDir::new().unwrap();
    let mut first_stake_pool_owner = thor::Wallet::default();
    let second_stake_pool_owner = thor::Wallet::default();
    let first_stake_pool = StakePool::new(&first_stake_pool_owner);

    let config = ConfigurationBuilder::new()
        .with_funds(vec![first_stake_pool_owner.to_initial_fund(1_000_000)])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .expect("Cannot start jormungandr");

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

    let params = ExplorerParams::new(
        query_complexity_limit.to_string(),
        query_depth_limit.to_string(),
        None,
    );
    let explorer_process = jormungandr.explorer(params);
    let explorer = explorer_process.client();

    let stake_pool_reg_fragment =
        fragment_builder.stake_pool_registration(&first_stake_pool_owner, &first_stake_pool);

    fragment_sender
        .send_fragment(
            &mut first_stake_pool_owner,
            stake_pool_reg_fragment,
            &jormungandr,
        )
        .expect("Error while sending registration certificate for first stake pool owner");

    let mut new_stake_pool = first_stake_pool.clone();
    let mut stake_pool_info = new_stake_pool.info_mut();

    stake_pool_info.reward_account = Some(AccountIdentifier::Single(
        second_stake_pool_owner
            .identifier()
            .into_public_key()
            .into(),
    ));

    let stake_pool_update_fragment = fragment_builder.stake_pool_update(
        vec![&first_stake_pool_owner],
        &first_stake_pool,
        &new_stake_pool,
    );

    jcli.fragment_sender(&jormungandr)
        .send(&stake_pool_update_fragment.encode())
        .assert_in_block();
    first_stake_pool_owner.confirm_transaction();

    let trans = explorer
        .transaction_certificates(stake_pool_update_fragment.hash().into())
        .expect("Non existing stake pool update transaction");

    assert!(trans.errors.is_none(), "{:?}", trans.errors.unwrap());

    let stake_pool_update_transaction = trans.data.unwrap().transaction;

    ExplorerVerifier::assert_transaction_certificates(
        stake_pool_update_fragment,
        stake_pool_update_transaction,
    )
    .unwrap();
}

#[test]
pub fn explorer_pool_retire_test() {
    let query_complexity_limit = 70;
    let query_depth_limit = 30;
    let temp_dir = TempDir::new().unwrap();
    let mut first_stake_pool_owner = thor::Wallet::default();
    let first_stake_pool = StakePool::new(&first_stake_pool_owner);

    let config = ConfigurationBuilder::new()
        .with_funds(vec![first_stake_pool_owner.to_initial_fund(1_000_000)])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .expect("Cannot start jormungandr");

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

    let params = ExplorerParams::new(
        query_complexity_limit.to_string(),
        query_depth_limit.to_string(),
        None,
    );
    let explorer_process = jormungandr.explorer(params);
    let explorer = explorer_process.client();

    let stake_pool_reg_fragment =
        fragment_builder.stake_pool_registration(&first_stake_pool_owner, &first_stake_pool);

    fragment_sender
        .send_fragment(
            &mut first_stake_pool_owner,
            stake_pool_reg_fragment,
            &jormungandr,
        )
        .expect("Error while sending registration certificate for first stake pool owner");
    let stake_pool_retire_fragment =
        fragment_builder.stake_pool_retire(vec![&first_stake_pool_owner], &first_stake_pool);

    fragment_sender
        .send_fragment(
            &mut first_stake_pool_owner,
            stake_pool_retire_fragment.clone(),
            &jormungandr,
        )
        .unwrap();

    let trans = explorer
        .transaction_certificates(stake_pool_retire_fragment.hash().into())
        .expect("Non existing stake pool update transaction");

    assert!(trans.errors.is_none(), "{:?}", trans.errors.unwrap());

    let stake_pool_retire_transaction = trans.data.unwrap().transaction;

    ExplorerVerifier::assert_transaction_certificates(
        stake_pool_retire_fragment,
        stake_pool_retire_transaction,
    )
    .unwrap();
}
