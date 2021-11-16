use crate::scenario::{Error, Result};
use jormungandr_testing_utils::testing::jormungandr::ConfiguredStarter;
use jormungandr_testing_utils::testing::jormungandr::JormungandrProcess;
use jormungandr_testing_utils::testing::network::controller::Controller;
use jormungandr_testing_utils::testing::network::SpawnParams;
use jormungandr_testing_utils::testing::node::configuration::legacy::NodeConfig as LegacyNodeConfig;
use jormungandr_testing_utils::testing::LegacyNodeConfigConverter;
use jormungandr_testing_utils::Version;

pub fn spawn_node(
    controller: &mut Controller,
    input_params: SpawnParams,
) -> Result<JormungandrProcess> {
    let alias = input_params.get_alias().clone();
    let mut starter = controller.make_starter_for(input_params)?;
    let (params, working_dir) = starter.build_configuration()?;
    let node_config = params.node_config().clone();

    let configurer_starter = ConfiguredStarter::new(&starter, params, working_dir);

    let mut command = configurer_starter.command();
    let process = command.spawn().map_err(Error::CannotSpawnNode)?;

    JormungandrProcess::new(
        process,
        &node_config,
        controller.settings().block0.clone(),
        None,
        alias,
    )
    .map_err(Into::into)
}

pub fn spawn_legacy_node(
    controller: &mut Controller,
    input_params: SpawnParams,
    version: &Version,
) -> Result<(JormungandrProcess, LegacyNodeConfig)> {
    let alias = input_params.get_alias().clone();
    let mut starter = controller.make_starter_for(input_params)?;
    let (params, working_dir) = starter.build_configuration()?;
    let node_config = params.node_config().clone();

    let configurer_starter =
        ConfiguredStarter::legacy(&starter, version.clone(), params, working_dir)?;

    let mut command = configurer_starter.command();
    let process = command.spawn().map_err(Error::CannotSpawnNode)?;

    let legacy_node_config =
        LegacyNodeConfigConverter::new(version.clone()).convert(&node_config)?;

    let process = JormungandrProcess::new(
        process,
        &legacy_node_config,
        controller.settings().block0.clone(),
        None,
        alias,
    )?;

    Ok((process, legacy_node_config))
}
