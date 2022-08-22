use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{explorer::configuration::ExplorerParams, Starter},
};

const BLOCK_QUERY_COMPLEXITY_LIMIT: u64 = 150;
const BLOCK_QUERY_DEPTH_LIMIT: u64 = 30;

#[test]
pub fn explorer_block0_test() {
    let jcli: JCli = Default::default();
    let jormungandr = Starter::new().start().unwrap();
    let rest_uri = jormungandr.rest_uri();
    let block0_id = jcli.rest().v0().tip(&rest_uri);
    let params = ExplorerParams::new(BLOCK_QUERY_COMPLEXITY_LIMIT, BLOCK_QUERY_DEPTH_LIMIT, None);
    let explorer_process = jormungandr.explorer(params);
    let explorer = explorer_process.client();

    explorer.block(block0_id).unwrap();
}

#[should_panic] //NPG-2899
#[test]
pub fn explorer_block_incorrect_id_test() {
    let incorrect_block_ids = vec![
        (
            "e1049ea45726f0b1fc473af54f706546b3331765abf89ae9e6a8333e49621641aa",
            "invalid hash size",
        ),
        (
            "e1049ea45726f0b1fc473af54f706546b3331765abf89ae9e6a8333e49621641a",
            "invalid hex encoding",
        ),
        (
            "e1049ea45726f0b1fc473af54f706546b3331765abf89ae9e6a8333e49621641",
            "Couldn't find block in the explorer",
        ),
    ];

    let jormungandr = Starter::new().start().unwrap();

    let explorer_process = jormungandr.explorer(ExplorerParams::default());
    let explorer = explorer_process.client();

    for (incorrect_block_id, error_message) in incorrect_block_ids {
        let response = explorer.block(incorrect_block_id.to_string());
        assert!(response.as_ref().unwrap().errors.is_some());
        assert!(response.as_ref().unwrap().data.is_none());
        assert!(response
            .unwrap()
            .errors
            .unwrap()
            .first()
            .unwrap()
            .message
            .contains(error_message));
    }
}
