use crate::common::configuration::{
    genesis_model::GenesisYaml, jormungandr_config::JormungandrConfig,
};
use crate::common::file_utils;
use crate::common::jcli_wrapper;
use crate::common::startup;

#[test]
pub fn test_genesis_decode_will_produce_correct_yaml_file() {
    let config = startup::ConfigurationBuilder::new().build();
    let expected_genesis_model = config.genesis_yaml;
    let genesis_block_file_path = startup::build_genesis_block(&expected_genesis_model);
    let decoded_genesis_block_file_path = file_utils::get_path_in_temp("decoded_genesis_file");
    jcli_wrapper::assert_genesis_decode(&genesis_block_file_path, &decoded_genesis_block_file_path);

    let content = file_utils::read_file(&decoded_genesis_block_file_path);
    let _model: GenesisYaml = match serde_yaml::from_str(&content) {
        Ok(v) => v,
        Err(e) => panic!(
            "genesis decode command cannot create correct yaml file{:?}",
            e
        ),
    };
}

#[test]
pub fn test_genesis_without_block0_date_fails_to_build() {
    let mut config = JormungandrConfig::new();
    config.genesis_yaml.blockchain_configuration.block0_date = None;
    jcli_wrapper::assert_genesis_encode_fails(&config.genesis_yaml, "missing field `block0_date`")
}
