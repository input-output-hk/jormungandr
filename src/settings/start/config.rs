use crate::settings::logging::LogFormat;
use cardano::hdwallet;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub secret_file: Option<PathBuf>,
    pub bft: Option<Bft>,
    pub genesis: Option<Genesis>,
    pub legacy_listen: Option<Vec<SocketAddr>>,
    pub grpc_listen: Option<Vec<SocketAddr>>,
    pub legacy_peers: Option<Vec<SocketAddr>>,
    pub grpc_peers: Option<Vec<SocketAddr>>,
    pub storage: Option<PathBuf>,
    pub logger: Option<ConfigLogSettings>,
    pub rest: Option<Rest>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Bft {
    pub constants: BftConstants,
    pub leaders: Vec<BftLeader>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BftConstants {
    /// stability time
    t: u64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Genesis {
    pub constant: GenesisConstants,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisConstants {
    /// stability time
    k: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BftLeader(pub hdwallet::XPub);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigLogSettings {
    pub verbosity: Option<u8>,
    pub format: Option<LogFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rest {
    pub listen: SocketAddr,
    pub pkcs12: PathBuf,
}
