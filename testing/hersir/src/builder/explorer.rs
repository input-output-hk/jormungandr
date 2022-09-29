use crate::{builder::NodeSetting, config::ExplorerTemplate};
use jormungandr_automation::{
    jormungandr::{explorer::configuration::ExplorerConfiguration, get_available_port, NodeAlias},
    utils::MultiaddrExtension,
};
use std::{collections::HashMap, path::Path};

pub fn generate_explorer(
    nodes: &HashMap<NodeAlias, NodeSetting>,
    explorer_template: &ExplorerTemplate,
) -> Result<ExplorerConfiguration, Error> {
    let settings = nodes
        .get(&explorer_template.connect_to)
        .ok_or_else(|| Error::CannotFindAlias(explorer_template.connect_to.clone()))?;

    Ok(ExplorerConfiguration {
        explorer_port: get_available_port(),
        explorer_listen_address: "127.0.0.1".to_string(),
        node_address: settings.config.p2p.public_address.clone().to_http_addr(),
        logs_dir: Some(Path::new("C:\\work\\iohk\\logs.txt").to_path_buf()),
        storage_dir: None,
        params: explorer_template.to_explorer_params(),
    })
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("cannot find alias '{0}' for any defined node")]
    CannotFindAlias(NodeAlias),
}
