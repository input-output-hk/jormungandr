use crate::common::file_utils;
use crate::common::jcli_wrapper;

#[test]
pub fn test_correct_hash_is_returned_for_correct_block() {
    let content = jcli_wrapper::assert_genesis_init();
    let path_to_yaml = file_utils::create_file_in_temp("init_file.yaml", &content);
    let path_to_output_block = file_utils::get_path_in_temp("block-0.bin");
    jcli_wrapper::assert_genesis_encode(&path_to_yaml, &path_to_output_block);
    jcli_wrapper::assert_genesis_hash(&path_to_output_block);
}

#[test]
pub fn test_correct_error_is_returned_for_non_existent_genesis_block() {
    let path_to_output_block = file_utils::get_path_in_temp("block-0.bin");
    jcli_wrapper::assert_genesis_hash_fails(&path_to_output_block, "file");
}
