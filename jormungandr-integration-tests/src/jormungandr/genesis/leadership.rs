use crate::common::{jcli_wrapper, jormungandr::ConfigurationBuilder, process_utils, startup};
use jormungandr_lib::interfaces::LeadershipLogStatus;

#[test]
pub fn test_leadership_logs_parent_hash_is_correct() {
    let faucet = startup::create_new_account_address();
    let (jormungandr, _) =
        startup::start_stake_pool(&[faucet], &[], &mut ConfigurationBuilder::new()).unwrap();

    process_utils::sleep(5);

    let rest_address = jormungandr.rest_address();
    let leadership_logs = jcli_wrapper::assert_rest_get_leadership_log(&rest_address);

    for leadership in leadership_logs.iter().take(10) {
        if let LeadershipLogStatus::Block {
            block,
            parent,
            chain_length: _,
        } = leadership.status()
        {
            let actual_block =
                jcli_wrapper::assert_rest_get_next_block_id(&parent.to_string(), &1, &rest_address);
            assert_eq!(actual_block, *block, "wrong parent block");
        }
    }
}
