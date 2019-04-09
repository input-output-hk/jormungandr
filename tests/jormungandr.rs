extern crate assert_cmd;
extern crate galvanic_test;
extern crate mktemp;

use galvanic_test::test_suite;

#[cfg(feature = "integration-test")]
test_suite! {

    mod file_assert;
    mod file_utils;
    mod resources_const;
    mod jormungandr_wrapper;
    mod jcli_wrapper;
    mod process_assert;
    mod process_utils;
    
    test test_jormungandr_node_starts_successfully() {

        let node_config = resources_const::NODE_CONFIG_FILE_PATH;
        let path_buf = file_utils::get_path_in_temp("block-0.bin");
        let path_to_output_block = path_buf.to_str().unwrap();

        process_assert::assert_process_exited_successfully(
            jcli_wrapper::run_genesis_encode_command_default(&path_to_output_block),
            "jcli genesis encode"
        );
       
        file_assert::assert_file_exists(&path_to_output_block);

        let process = jormungandr_wrapper::start_jormungandr_node(&node_config,&path_to_output_block)
                .spawn()
                .expect("failed to execute 'start jormungandr node'");
        let _guard = process_utils::ProcessKillGuard::new(process);
        
        process_utils::run_process_until_exited_successfully(
            jcli_wrapper::run_rest_stats_command_default(),
            2,
            5,
            "get stats from jormungandr node", 
            "jormungandr node is not up"
        );       
    }
}
