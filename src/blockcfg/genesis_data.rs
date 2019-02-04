pub struct GenesisData {
    pub start_time: std::time::SystemTime,
    pub slot_duration: std::time::Duration,
    pub epoch_stability_depth: usize,
}

impl GenesisData {
    pub fn parse<R: std::io::BufRead>(reader: R) -> Result<Self, impl std::error::Error> {
        unimplemented!()
    }
}
