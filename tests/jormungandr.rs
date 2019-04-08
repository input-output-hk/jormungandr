extern crate assert_cmd;
extern crate galvanic_test;
extern crate mktemp;
extern crate regex;

use galvanic_test::test_suite;

#[cfg(feature = "integration-test")]
test_suite! {

    use regex::Regex;
    use std::thread;
    use std::time;
    use std::process::{Command, Stdio};
    use std::io::{BufRead, BufReader};
    use std::fs::File;

    mod file_assert;
    mod file_utils;
    mod resources_const;
    mod jormungandr_wrapper;
    mod jcli_wrapper;

    test test_jormungandr_node_starts_successfully() {

        let node_config = resources_const::NODE_CONFIG_FILE_PATH;
        let path_buf = file_utils::get_path_in_temp("block-0.bin");
        let path_to_output_block = path_buf.to_str().unwrap();

       let mut process =  jcli_wrapper::run_genesis_encode_command_default(&path_to_output_block)
            .spawn()
            .expect("failed to execute genesis encode command");

        let exit_code = process
                 .wait()
                 .expect("failed to wait on  genesis encode command finish");

        assert!(exit_code.success());
        file_assert::assert_file_exists(&path_to_output_block);

        let mut process = jormungandr_wrapper::start_jormungandr_node(&node_config,&path_to_output_block)
            .spawn()
            .expect("failed to execute 'start jormungandr node'");

       use std::{thread, time};

       let ten_millis = time::Duration::from_millis(3000);
       thread::sleep(ten_millis);

       process.kill();
    }
}
