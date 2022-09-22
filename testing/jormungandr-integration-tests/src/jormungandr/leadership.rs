use crate::startup;
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{ConfigurationBuilder, JormungandrProcess, StartupVerificationMode},
};
use jormungandr_lib::interfaces::LeadershipLogStatus;
use std::time::Duration;

#[test]
fn verify_genesis_praos_leadership_logs_parent_hash() {
    let faucet = thor::Wallet::default();
    let (jormungandr, _) =
        startup::start_stake_pool(&[faucet], &[], &mut ConfigurationBuilder::new()).unwrap();

    verify_leadership_logs_parent_hash(jormungandr);
}

#[test]
fn verify_bft_leadership_logs_parent_hash() {
    let jormungandr = startup::start_bft(
        vec![&thor::Wallet::default()],
        &mut ConfigurationBuilder::new(),
    )
    .unwrap();

    verify_leadership_logs_parent_hash(jormungandr);
}

fn verify_leadership_logs_parent_hash(jormungandr: JormungandrProcess) {
    jormungandr
        .wait_for_bootstrap(&StartupVerificationMode::Rest, Duration::from_secs(10))
        .unwrap();

    // Give the node some time to produce blocks
    std::thread::sleep(Duration::from_secs(5));

    let jcli = JCli::default();

    let leadership_logs = jcli.rest().v0().leadership_log(jormungandr.rest_uri());

    // leadership logs are fetched in reverse order (newest first)
    for leadership in leadership_logs.iter().take(10).rev() {
        if let LeadershipLogStatus::Block { block, parent, .. } = leadership.status() {
            let actual_blocks =
                jcli.rest()
                    .v0()
                    .block()
                    .next(parent.to_string(), 1, jormungandr.rest_uri());
            let actual_block = actual_blocks.first().unwrap();
            
            assert_eq!(actual_block, block, "wrong parent block");
        }
    }
}
