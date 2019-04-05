extern crate assert_cmd;
extern crate galvanic_test;
extern crate mktemp;

use galvanic_test::test_suite;

#[cfg(feature = "integration-test")]
test_suite! {

    use std::process::Command;
    use mktemp::Temp;
    use assert_cmd::prelude::{OutputOkExt,OutputAssertExt};
    use std::path::PathBuf;

    mod file_assert;

    const GENESIS_YAML_FILE_PATH: &str = "./tests/resources/genesis/genesis.yaml";

    fixture genesis_fixture() -> PathBuf {
        setup(&mut self) {
             let temp_dir = Temp::new_dir().unwrap();
             let mut path = temp_dir.to_path_buf();
             path.push("block-0.bin");
             println!("Setup: location for output block file: {}",&path.to_str().unwrap());
             temp_dir.release();
             path
        }
    }

    test test_genesis_block_is_built_from_corect_yaml(genesis_fixture) {

        let path_to_output_block = genesis_fixture.val.to_str().unwrap();

        Command::new("jcli")
            .arg("genesis")
            .arg("encode")
            .arg("--input")
            .arg(&GENESIS_YAML_FILE_PATH)
            .arg("--output")
            .arg(&path_to_output_block)
            .unwrap()
            .assert()
            .success();

       file_assert::assert_file_exists_and_not_empty(&path_to_output_block);
    }
}
