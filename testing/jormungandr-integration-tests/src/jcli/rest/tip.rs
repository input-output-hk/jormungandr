use crate::startup::SingleNodeTestBootstrapper;
use assert_fs::TempDir;
use jormungandr_automation::{jcli::JCli, jormungandr::NodeConfigBuilder};

#[test]
pub fn test_correct_id_is_returned_for_block_tip_if_only_genesis_block_exists() {
    let jcli: JCli = Default::default();
    let temp_dir = TempDir::new().unwrap();
    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .build()
        .start_node(temp_dir)
        .unwrap();
    let block_id = jcli.rest().v0().tip(jormungandr.rest_uri());

    assert_ne!(&block_id, "", "empty block hash");
}

#[test]
pub fn test_correct_error_is_returned_for_incorrect_path() {
    let jcli: JCli = Default::default();
    let config = NodeConfigBuilder::default().build();
    let incorrect_uri = format!("http://{}/api/api", config.rest.listen);

    jcli.rest()
        .v0()
        .tip_expect_fail(incorrect_uri, "tcp connect error");
}
