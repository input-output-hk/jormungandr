mod genesis {

    use galvanic_test::test_suite;

    test_suite! {

        extern crate integration_tests;
        use integration_tests::assert_file_exists_and_not_empty;
        use assert_cmd::prelude::*;
        use std::process::*;
        use std::fs;

        const GENESIS_OUTPUT_BLOCK_FILE_PATH: &str = "./block-0.bin";
        const GENESIS_YAML_FILE_PATH: &str = "./resources/genesis/genesis.yaml";

        fixture genesis_fixture() -> i32 {
            // Passing setup
            setup(&mut self) {
                0
            }

            // Cleaning file
            tear_down(&self)  {
                let file_to_clean = GENESIS_OUTPUT_BLOCK_FILE_PATH;
                let message = format!("cannot remove file {}",&file_to_clean);
                fs::remove_file(&file_to_clean).expect(&message);
            }
        }

        test test_genesis_block_is_built_from_corect_yaml() {

            Command::new("jcli")
                .arg("genesis")
                .arg("encode")
                .arg("--input")
                .arg(&GENESIS_YAML_FILE_PATH)
                .arg("--output")
                .arg(&GENESIS_OUTPUT_BLOCK_FILE_PATH)
                .unwrap()
                .assert()
                .success();

            assert_file_exists_and_not_empty(&GENESIS_OUTPUT_BLOCK_FILE_PATH);
        }

    }
}
