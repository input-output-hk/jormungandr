use crate::common::jcli_wrapper;

use assert_fs::prelude::*;
use assert_fs::TempDir;

#[test]
pub fn test_genesis_block_is_built_from_init_yaml() {
    let content = jcli_wrapper::assert_genesis_init();
    let temp_dir = TempDir::new().unwrap();
    let yaml_file = temp_dir.child("init_file.yaml");
    yaml_file.write_str(&content).unwrap();
    let block_file = temp_dir.child("block-0.bin");
    jcli_wrapper::assert_genesis_encode(yaml_file.path(), &block_file);
}
