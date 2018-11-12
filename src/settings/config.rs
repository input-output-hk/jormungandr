use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub secret_file: Option<PathBuf>,
    pub bft: Option<Bft>,
    pub genesis: Option<Genesis>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Bft {
    constants: BftConstants,
    leaders: Vec<BftLeader>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BftConstants {
    /// stability time
    t: u64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Genesis {
    constant: GenesisConstants,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisConstants {
    /// stability time
    k: u64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BftLeader(String);
