extern crate assert_cmd;
extern crate galvanic_test;
extern crate mktemp;

use galvanic_test::test_suite;

#[cfg(feature = "integration-test")]
test_suite! {

    use assert_cmd::prelude::{OutputOkExt,OutputAssertExt};
    use std::path::PathBuf;
    mod file_assert;
    mod file_utils;
    mod configuration;
    mod jcli_wrapper;

    fixture genesis_fixture() -> PathBuf {
        setup(&mut self) {
             let mut path = file_utils::get_path_in_temp("block-0.bin");
             println!("Setup: location for output block file: {:?}",path);
             path
        }
    }

    test test_genesis_block_is_built_from_corect_yaml(genesis_fixture) {

        let input_yaml_file_path = configuration::get_genesis_yaml_path();
        let path_to_output_block = genesis_fixture.val;


        jcli_wrapper::run_genesis_encode_command(&input_yaml_file_path,&path_to_output_block)
            .unwrap()
            .assert()
            .success();

       file_assert::assert_file_exists_and_not_empty(path_to_output_block);
    }
}
