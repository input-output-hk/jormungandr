use assert_fs::{prelude::*, TempDir};
use jormungandr_automation::jcli::JCli;

#[test]
pub fn test_genesis_block_is_built_from_init_yaml() {
    let jcli: JCli = Default::default();

    let content = jcli.genesis().init();
    let temp_dir = TempDir::new().unwrap();
    let yaml_file = temp_dir.child("init_file.yaml");
    yaml_file.write_str(&content).unwrap();
    let block_file = temp_dir.child("block-0.bin");
    jcli.genesis().encode(yaml_file.path(), &block_file);
}
