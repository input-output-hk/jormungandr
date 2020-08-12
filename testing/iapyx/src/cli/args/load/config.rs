use jortestkit::load::Configuration;
use std::path::PathBuf;

pub struct IapyxLoadConfig {
    pub config: Configuration,
    pub measure: bool,
    pub address: String,
    pub mnemonics_file: PathBuf,
}

impl IapyxLoadConfig {
    pub fn new(
        config: Configuration,
        measure: bool,
        address: String,
        mnemonics_file: PathBuf,
    ) -> Self {
        Self {
            config,
            measure,
            address,
            mnemonics_file,
        }
    }
}
