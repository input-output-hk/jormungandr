use crate::common::{file_utils, jcli_wrapper};

#[test]
pub fn test_genesis_block_is_built_from_init_yaml() {
    let content = jcli_wrapper::assert_genesis_init();
    let path_to_yaml = file_utils::create_file_in_temp("init_file.yaml", &content);
    let path_to_output_block = file_utils::get_path_in_temp("block-0.bin");
    jcli_wrapper::assert_genesis_encode(&path_to_yaml, &path_to_output_block);
}
