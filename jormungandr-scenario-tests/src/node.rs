use crate::{
    scenario::{settings::NodeSetting, NodeBlock0},
    Context, NodeAlias,
};
use bawawa::Process;
use mktemp::Temp;
use rand_core::RngCore;

error_chain! {
    errors {
        CannotCreateTemporaryDirectory {
            description("Cannot create a temporary directory")
        }

        CannotSpawnNode {
            description("Cannot spawn the node"),
        }
    }
}

pub struct Node {
    alias: NodeAlias,

    #[allow(unused)]
    temp_dir: Temp,

    process: Process,
}

const NODE_CONFIG: &str = "node_config.yaml";
const NODE_SECRET: &str = "node_secret.yaml";

impl Node {
    pub fn alias(&self) -> &NodeAlias {
        &self.alias
    }

    pub fn process(&self) -> &Process {
        &self.process
    }

    pub fn process_mut(&mut self) -> &mut Process {
        &mut self.process
    }

    pub fn spawn<R: RngCore>(
        context: &Context<R>,
        alias: &str,
        node_settings: &NodeSetting,
        block0: NodeBlock0,
    ) -> Result<Self> {
        let mut command = context.jormungandr().clone();
        let temp_dir = Temp::new_dir().chain_err(|| ErrorKind::CannotCreateTemporaryDirectory)?;

        let config_file = {
            let mut dir = temp_dir.clone().release();
            dir.push(NODE_CONFIG);
            dir
        };
        let config_secret = {
            let mut dir = temp_dir.clone().release();
            dir.push(NODE_SECRET);
            dir
        };

        serde_yaml::to_writer(
            std::fs::File::create(&config_file)
                .chain_err(|| format!("Cannot create file {:?}", config_file))?,
            node_settings.config(),
        )
        .chain_err(|| format!("cannot write in {:?}", config_file))?;

        serde_yaml::to_writer(
            std::fs::File::create(&config_secret)
                .chain_err(|| format!("Cannot create file {:?}", config_secret))?,
            node_settings.secrets(),
        )
        .chain_err(|| format!("cannot write in {:?}", config_secret))?;

        command.arguments(&[
            "--config",
            &config_file.display().to_string(),
            "--secret",
            &config_secret.display().to_string(),
        ]);

        match block0 {
            NodeBlock0::File(path) => {
                command.arguments(&["--genesis-block", &path.display().to_string()]);
            }
            NodeBlock0::Hash(hash) => {
                command.arguments(&["--genesis-block-hash", &hash.to_string()]);
            }
        }

        let process = Process::spawn(command).chain_err(|| ErrorKind::CannotSpawnNode)?;

        Ok(Node {
            alias: alias.into(),

            temp_dir,

            process,
        })
    }
}
