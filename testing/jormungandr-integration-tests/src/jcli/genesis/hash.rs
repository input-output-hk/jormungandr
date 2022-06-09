use assert_fs::{prelude::*, TempDir};
use jormungandr_automation::jcli::JCli;

#[test]
pub fn test_correct_hash_is_returned_for_correct_block() {
    let jcli: JCli = Default::default();
    let content = jcli.genesis().init();
    let temp_dir = TempDir::new().unwrap();
    let yaml_file = temp_dir.child("init_file.yaml");
    yaml_file.write_str(&content).unwrap();
    let block_file = temp_dir.child("block-0.bin");

    jcli.genesis().encode(yaml_file.path(), &block_file);
    jcli.genesis().hash(block_file.path());
}

#[test]
pub fn test_correct_error_is_returned_for_non_existent_genesis_block() {
    let temp_dir = TempDir::new().unwrap();
    let block_file = temp_dir.child("block-0.bin");
    let jcli: JCli = Default::default();
    jcli.genesis().hash_expect_fail(block_file.path(), "file");
}
