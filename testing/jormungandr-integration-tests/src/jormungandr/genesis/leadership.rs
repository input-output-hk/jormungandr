use crate::common::{
    jcli::JCli,
    jormungandr::{ConfigurationBuilder, StartupVerificationMode},
    startup,
};
use jormungandr_lib::interfaces::LeadershipLogStatus;
use std::time::Duration;

#[test]
pub fn test_leadership_logs_parent_hash_is_correct() {
    let faucet = startup::create_new_account_address();
    let jcli: JCli = Default::default();
    let (jormungandr, _) =
        startup::start_stake_pool(&[faucet], &[], &mut ConfigurationBuilder::new()).unwrap();

    jormungandr
        .wait_for_bootstrap(&StartupVerificationMode::Rest, Duration::from_secs(10))
        .unwrap();

    // Give the node some time to produce blocks
    std::thread::sleep(Duration::from_secs(5));

    let leadership_logs = jcli.rest().v0().leadership_log(jormungandr.rest_uri());

    // leadership logs are fetched in reverse order (newest first)
    for leadership in leadership_logs.iter().take(10).rev() {
        if let LeadershipLogStatus::Block {
            block,
            parent,
            chain_length: _,
        } = leadership.status()
        {
            let actual_block =
                jcli.rest()
                    .v0()
                    .block()
                    .next(parent.to_string(), 1, jormungandr.rest_uri());
            assert_eq!(actual_block, *block, "wrong parent block");
        }
    }
}
