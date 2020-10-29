use crate::common::{jcli::JCli, jormungandr::ConfigurationBuilder, startup};
use jormungandr_lib::{crypto::hash::Hash, interfaces::LeadershipLogStatus};
use jortestkit::process::sleep;
use std::str::FromStr;

#[test]
pub fn test_leadership_logs_parent_hash_is_correct() {
    let faucet = startup::create_new_account_address();
    let jcli: JCli = Default::default();
    let (jormungandr, _) =
        startup::start_stake_pool(&[faucet], &[], &mut ConfigurationBuilder::new()).unwrap();

    sleep(5);

    let leadership_logs = jcli.rest().v0().leadership_log(jormungandr.rest_uri());

    for leadership in leadership_logs.iter().take(10) {
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
