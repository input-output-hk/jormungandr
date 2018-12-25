use cardano::hdwallet;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

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
#[serde(rename_all = "lowercase")]
/// Format of the logger.
pub enum LogFormat {
    Plain,
    Json,
}

impl FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cleared = s.trim().to_lowercase();
        if cleared == "plain" {
            Ok(LogFormat::Plain)
        } else if cleared == "json" {
            Ok(LogFormat::Json)
        } else {
            let mut msg = "unknown format ".to_string();
            msg.push_str(&cleared);
            Err(msg)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigLogSettings {
    pub verbosity: Option<u8>,
    pub format: Option<LogFormat>,
}
